use std::time::Duration;

use bevy::{platform::collections::HashSet, prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    faction::Factions,
    physics::location::{Location, SectorUnit, systems::Anchor},
    state::GameState,
    structure::station::station_builder::STATION_LOAD_DISTANCE,
};

use crate::persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded};

use super::super::{SystemItem, UniverseSystems};

const FACTION_STATION_ID: &str = "cosmos:faction_station";

fn spawn_npc_stations(
    q_players: Query<&Location, With<Anchor>>,
    mut commands: Commands,
    mut systems: ResMut<UniverseSystems>,
    factions: Res<Factions>,
) {
    let mut generated_stations = HashSet::new();

    for player_loc in q_players.iter() {
        let Some(system) = systems.system_mut(player_loc.get_system_coordinates()) else {
            continue;
        };

        for (station_rot, station_loc, station) in system
            .iter()
            .flat_map(|x| match &x.item {
                SystemItem::NpcStation(s) => Some((x.rotation, x.location, s)),
                _ => None,
            })
            .filter(|(_, x, _)| !system.is_sector_generated_for(x.sector(), FACTION_STATION_ID))
        {
            if generated_stations.contains(&station_loc.sector()) {
                continue;
            }

            let sector_diff = (station_loc.sector() - player_loc.sector()).abs();
            if sector_diff.max_element() > STATION_LOAD_DISTANCE as SectorUnit {
                continue;
            }

            let bp_name = &station.build_type;

            let mut ecmds = commands.spawn(NeedsBlueprintLoaded {
                path: format!("default_blueprints/faction/stations/{bp_name}.bp"),
                rotation: station_rot,
                spawn_at: station_loc,
            });

            if let Some(fac) = factions.from_id(&station.faction) {
                ecmds.insert(fac.id());
            } else {
                error!("No faction for NPC station ({:?})!", station.faction);
            }

            info!("Generating NPC station @ {station_loc}");

            generated_stations.insert(station_loc.sector());
        }

        for &generated_station in &generated_stations {
            system.mark_sector_generated_for(generated_station, FACTION_STATION_ID);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        spawn_npc_stations
            .run_if(on_timer(Duration::from_secs(1)))
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing)),
    );
}
