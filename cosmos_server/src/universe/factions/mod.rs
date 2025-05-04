use std::time::Duration;

use bevy::{
    log::{error, info},
    prelude::{App, Commands, EventReader, IntoSystemConfigs, Query, Res, ResMut, Update, Vec3, With, in_state},
    time::common_conditions::on_timer,
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    faction::Factions,
    physics::location::{Location, SECTOR_DIMENSIONS, SYSTEM_SECTORS, Sector, SectorUnit},
    state::GameState,
    structure::station::station_builder::STATION_LOAD_DISTANCE,
    utils::quat_math::random_quat,
};
use rand::{Rng, seq::IteratorRandom};

use crate::{
    init::init_world::ServerSeed,
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
    rng::get_rng_for_sector,
    universe::generation::{GenerateSystemEvent, SystemGenerationSet, SystemItem, UniverseSystems},
};

const FACTION_STATION_ID: &str = "cosmos:faction_station";

fn spawn_faction_stations(
    q_players: Query<&Location, With<Player>>,
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

        for &generated_shop in &generated_stations {
            system.mark_sector_generated_for(generated_shop, FACTION_STATION_ID);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        spawn_faction_stations
            .run_if(on_timer(Duration::from_secs(1)))
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .run_if(in_state(GameState::Playing)),
    );
}
