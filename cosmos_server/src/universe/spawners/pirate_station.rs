use std::time::Duration;

use bevy::{
    log::{error, info},
    prelude::{App, Commands, IntoSystemConfigs, Query, Res, ResMut, Update, With, in_state},
    time::common_conditions::on_timer,
    platform::collections::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    faction::Factions,
    physics::location::{Location, SectorUnit},
    registry::Registry,
    state::GameState,
    structure::station::station_builder::STATION_LOAD_DISTANCE,
};

use crate::{
    ai::pirate::station::PirateStation,
    loot::{LootTable, NeedsLootGenerated},
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
};

use super::super::{SystemItem, UniverseSystems};

const PIRATE_STATION_ID: &str = "cosmos:pirate_station";

fn spawn_pirate_stations(
    q_players: Query<&Location, With<Player>>,
    mut commands: Commands,
    mut systems: ResMut<UniverseSystems>,
    factions: Res<Factions>,
    loot: Res<Registry<LootTable>>,
) {
    let mut generated_stations = HashSet::new();

    for player_loc in q_players.iter() {
        let Some(system) = systems.system_mut(player_loc.get_system_coordinates()) else {
            continue;
        };

        for (station_rot, station_loc, station) in system
            .iter()
            .flat_map(|x| match &x.item {
                SystemItem::PirateStation(s) => Some((x.rotation, x.location, s)),
                _ => None,
            })
            .filter(|(_, x, _)| !system.is_sector_generated_for(x.sector(), PIRATE_STATION_ID))
        {
            if generated_stations.contains(&station_loc.sector()) {
                continue;
            }

            let sector_diff = (station_loc.sector() - player_loc.sector()).abs();
            if sector_diff.max_element() > STATION_LOAD_DISTANCE as SectorUnit {
                continue;
            }

            let bp_name = &station.build_type;

            let mut ecmds = commands.spawn((
                PirateStation,
                NeedsLootGenerated::from_loot_id("cosmos:pirate_station_0", &loot).expect("Missing pirate_station_0.json"),
                NeedsBlueprintLoaded {
                    path: format!("default_blueprints/pirate/stations/{bp_name}.bp"),
                    rotation: station_rot,
                    spawn_at: station_loc,
                },
            ));

            if let Some(fac) = factions.from_name("Pirate") {
                ecmds.insert(fac.id());
            } else {
                error!("No pirate faction!");
            }

            info!("Generating Pirate station @ {station_loc}");

            generated_stations.insert(station_loc.sector());
        }

        for &generated_station in &generated_stations {
            system.mark_sector_generated_for(generated_station, PIRATE_STATION_ID);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        spawn_pirate_stations
            .run_if(on_timer(Duration::from_secs(1)))
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing)),
    );
}
