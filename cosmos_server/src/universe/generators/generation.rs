//! Responsible for the generation of the stars

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet},
    physics::location::{Location, SystemCoordinate},
    state::GameState,
};
use std::{collections::HashSet, fs, time::Duration};

use crate::{
    persistence::loading::LoadingBlueprintSystemSet,
    universe::{UniverseSystem, UniverseSystems},
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The ordering that a system should be generated in a galaxy
pub enum SystemGenerationSet {
    /// The events to generate a system are sent
    SendMessages,
    /// Add stars to the system
    Star,
    /// Add planets to the system
    Planet,
    /// Add asteroids to the system
    Asteroid,
    /// Adds faction locations to the system
    FactionStations,
    /// Add stations to the system
    Shop,
    /// Adds pirate stations to the system
    PirateStation,
    /// Comptues the danger values for the system
    ComputeDanger,
}

#[derive(Message, Debug)]
/// Sent whenever a [`UniverseSystem`] needs to be generated.
///
/// Generate it via accessing the [`UniverseSystems`] resource. Make sure to order your system
/// within the [`SystemGenerationSet`] in the proper set.
pub struct GenerateSystemMessage {
    /// The system's coordinate - used to access the system via the resource [`UniverseSystems`]
    pub system: SystemCoordinate,
}

fn load_saved_universe_system(system: SystemCoordinate) -> Option<UniverseSystem> {
    let Ok(universe_system) = fs::read(format!("world/systems/{},{},{}.usys", system.x(), system.y(), system.z())) else {
        return None;
    };

    Some(cosmos_encoder::deserialize(&universe_system).expect("Error parsing world system!"))
}

fn save_universe_systems(systems: Res<UniverseSystems>) {
    for (system_coord, system) in systems.systems.iter() {
        let serialized = cosmos_encoder::serialize(system);
        let _ = fs::create_dir("world/systems");

        fs::write(
            format!("world/systems/{},{},{}.usys", system_coord.x(), system_coord.y(), system_coord.z()),
            serialized,
        )
        .unwrap_or_else(|_| panic!("Failed to save universe system at -- {system_coord}"));
    }
}

const SPAWN_SYSTEM_LOCATION: Location = Location::ZERO;

fn unload_universe_systems_without_players(q_players: Query<&Location, With<Player>>, mut universe_systems: ResMut<UniverseSystems>) {
    let systems = q_players
        .iter()
        .chain(&[SPAWN_SYSTEM_LOCATION])
        .map(|x| SystemCoordinate::from_sector(x.sector()))
        .collect::<HashSet<SystemCoordinate>>();

    universe_systems.systems.retain(|k, _| systems.contains(k));
}

fn load_universe_systems_near_players(
    mut universe_systems: ResMut<UniverseSystems>,
    mut evw_generate_system: MessageWriter<GenerateSystemMessage>,
    q_players: Query<&Location, With<Player>>,
) {
    let mut sectors_todo = HashSet::new();

    for p_loc in q_players.iter().chain(&[SPAWN_SYSTEM_LOCATION]) {
        let system = p_loc.get_system_coordinates();

        if universe_systems.system(system).is_some() {
            continue;
        }

        if let Some(universe_system) = load_saved_universe_system(system) {
            universe_systems.systems.insert(universe_system.coordinate, universe_system);
        } else {
            sectors_todo.insert(system);
        }
    }

    if sectors_todo.is_empty() {
        return;
    }

    for &system_coordinate in &sectors_todo {
        universe_systems
            .systems
            .insert(system_coordinate, UniverseSystem::new(system_coordinate));
    }

    info!("Triggering system generation for {sectors_todo:?}");
    evw_generate_system.write_batch(sectors_todo.into_iter().map(|system| GenerateSystemMessage { system }));
}

fn recompute_sector_danger(mut evr_generate_system: MessageReader<GenerateSystemMessage>, mut systems: ResMut<UniverseSystems>) {
    for ev in evr_generate_system.read() {
        let Some(system) = systems.system_mut(ev.system) else {
            continue;
        };

        info!("Computing system {}'s danger...", system.coordinate);

        system.recompute_all_danger();

        info!("Done computing system {}'s danger.", system.coordinate);
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            SystemGenerationSet::SendMessages,
            SystemGenerationSet::Star,
            SystemGenerationSet::Planet,
            SystemGenerationSet::Asteroid,
            SystemGenerationSet::FactionStations,
            SystemGenerationSet::Shop,
            SystemGenerationSet::PirateStation,
            SystemGenerationSet::ComputeDanger,
        )
            .in_set(NetworkingSystemsSet::Between)
            .before(LoadingBlueprintSystemSet::BeginLoadingBlueprints)
            .chain(),
    );

    app.add_systems(
        FixedUpdate,
        (
            (load_universe_systems_near_players, unload_universe_systems_without_players).chain(),
            save_universe_systems.run_if(on_timer(Duration::from_secs(10))),
        )
            .run_if(in_state(GameState::Playing))
            .in_set(SystemGenerationSet::SendMessages),
    )
    .add_systems(
        FixedUpdate,
        recompute_sector_danger
            .in_set(SystemGenerationSet::ComputeDanger)
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<UniverseSystems>()
    .add_message::<GenerateSystemMessage>();
}
