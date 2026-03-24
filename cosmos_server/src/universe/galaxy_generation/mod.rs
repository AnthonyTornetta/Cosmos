//! Responsible for the generation of the overall Galaxy.
//!
//! Sets up things such as stars

use crate::{init::init_world::ServerSeed, persistence::WorldRoot, rng::get_rng_for_sector};
use bevy::{
    platform::collections::HashSet,
    prelude::*,
    time::common_conditions::{on_real_timer, on_timer},
};
use cosmos_core::{
    entities::player::Player,
    netty::cosmos_encoder,
    physics::location::{Location, SYSTEM_SECTORS, Sector, SectorUnit, SystemCoordinate, SystemUnit},
    state::GameState,
    time::UniverseTimestamp,
    universe::star::{MAX_TEMPERATURE, MIN_TEMPERATURE, Star},
};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::{PI, TAU},
    fs,
    time::Duration,
};

use super::{Galaxy, GalaxyStar};

mod factions_placement;
mod star;

fn populate_galaxy(mut commands: Commands, mut mw_generate_galaxy: MessageWriter<GenerateGalaxyMessage>, world_root: Res<WorldRoot>) {
    let mut ecmds = commands.spawn(Name::new("Galaxy"));
    let galaxy = load_galaxy(&world_root).unwrap_or_else(|| {
        mw_generate_galaxy.write(GenerateGalaxyMessage(ecmds.id()));
        Galaxy::default()
    });

    ecmds.insert(galaxy);
}

#[derive(Serialize, Deserialize, Default)]
struct GameInfo {
    timestamp: UniverseTimestamp,
}

fn load_game_info(world_root: &WorldRoot) -> Option<GameInfo> {
    let Ok(info) = fs::read(world_root.path_for("game_info.json")) else {
        return None;
    };

    Some(serde_json::de::from_slice(&info).expect("Unable to deserialize game info"))
}

fn save_game_info(game_info: &GameInfo, world_root: &WorldRoot) {
    let encoded = serde_json::ser::to_string_pretty(&game_info).unwrap();
    fs::write(world_root.path_for("game_info.json"), encoded).expect("Error saving game info");
}

fn init_game_info(mut commands: Commands, world_root: Res<WorldRoot>) {
    let info = load_game_info(&world_root).unwrap_or_default();

    commands.insert_resource(info.timestamp);
}

fn load_galaxy(world_root: &WorldRoot) -> Option<Galaxy> {
    let Ok(galaxy_bytes) = fs::read(world_root.path_for("galaxy.bin")) else {
        return None;
    };

    Some(cosmos_encoder::deserialize(&galaxy_bytes).expect("Unable to deserialize galaxy"))
}

fn save_galaxy(galaxy: &Galaxy, world_root: &WorldRoot) {
    let encoded = cosmos_encoder::serialize(&galaxy);
    fs::write(world_root.path_for("galaxy.bin"), encoded).expect("Error saving galaxy");
}

fn save_game_info_on_tick(timestamp: Res<UniverseTimestamp>, world_root: Res<WorldRoot>) {
    save_game_info(&GameInfo { timestamp: *timestamp }, &world_root);
}

// this shouldnt be here, but idc
fn advance_timestamp(mut timestamp: ResMut<UniverseTimestamp>) {
    timestamp.tick();
}

#[derive(Message)]
/// Sent when a galaxy should be generated - contains the galaxy's entity
pub struct GenerateGalaxyMessage(Entity);

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash, SystemSet)]
enum GalaxyGenerationOrder {
    Begin,
    Empty,
    BlackHolePlacement,
    StarsGeneration,
    FactionsPlacement,
    PlayerSpawnCreation,
    Done,
}

/// The schedule the galaxy is generated in
pub const GENERATE_GALAXY_SCHEDULE: OnEnter<GameState> = OnEnter(GameState::Playing);

fn save_on_done_generating(
    q_galaxy: Query<&Galaxy>,
    world_root: Res<WorldRoot>,
    mut mr_generate_galaxy: MessageReader<GenerateGalaxyMessage>,
) {
    for m in mr_generate_galaxy.read() {
        let Ok(galaxy) = q_galaxy.get(m.0) else {
            return;
        };
        save_galaxy(&galaxy, &world_root);
    }
}

pub(super) fn register(app: &mut App) {
    star::register(app);
    factions_placement::register(app);

    app.configure_sets(
        GENERATE_GALAXY_SCHEDULE,
        (
            GalaxyGenerationOrder::Begin,
            GalaxyGenerationOrder::Empty,
            GalaxyGenerationOrder::BlackHolePlacement,
            GalaxyGenerationOrder::StarsGeneration,
            GalaxyGenerationOrder::FactionsPlacement,
            GalaxyGenerationOrder::PlayerSpawnCreation,
            GalaxyGenerationOrder::Done,
        )
            .chain(),
    );

    app.add_systems(OnExit(GameState::PostLoading), init_game_info)
        .add_systems(
            GENERATE_GALAXY_SCHEDULE,
            (
                populate_galaxy.in_set(GalaxyGenerationOrder::Begin),
                save_on_done_generating.in_set(GalaxyGenerationOrder::Done),
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                save_game_info_on_tick
                    .run_if(on_real_timer(Duration::from_secs(5)))
                    .run_if(in_state(GameState::Playing)),
                advance_timestamp
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(any_with_component::<Player>),
            ),
        )
        .register_type::<Galaxy>()
        .add_message::<GenerateGalaxyMessage>();
}
