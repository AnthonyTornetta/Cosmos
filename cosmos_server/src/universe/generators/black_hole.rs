//! Contains server-side logic for stars

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    persistence::LoadingDistance,
    physics::location::{Location, SECTOR_DIMENSIONS},
    state::GameState,
    universe::black_hole::BlackHole,
};

use crate::persistence::{
    SerializedData,
    saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
};

use super::{
    super::{Galaxy, SystemItem, UniverseSystems},
    generation::{GenerateSystemMessage, SystemGenerationSet},
};

#[derive(Component)]
struct SpawnPos(Location);

fn load_black_holes_in_universe(q_galaxy: Query<&Galaxy>, mut commands: Commands, q_black_holes: Query<&Location, With<BlackHole>>) {
    let Ok(galaxy) = q_galaxy.single() else {
        return;
    };

    if !q_black_holes.is_empty() {
        return;
    }

    let black_hole_loc = galaxy.black_hole_loc();

    commands.spawn((
        BlackHole {
            radius: SECTOR_DIMENSIONS / 3.0,
        },
        SpawnPos(black_hole_loc),
        black_hole_loc,
        Name::new("Black Hole"),
        Velocity::zero(),
        LoadingDistance::infinite(),
    ));
}

fn on_save_black_hole(mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<BlackHole>)>) {
    for mut data in query.iter_mut() {
        data.set_should_save(false);
    }
}

fn generate_black_hole(
    mut evr_generate_system: MessageReader<GenerateSystemMessage>,
    mut universe_systems: ResMut<UniverseSystems>,
    q_galaxy: Query<&Galaxy>,
) {
    for ev in evr_generate_system.read() {
        let system = ev.system;

        let Ok(galaxy) = q_galaxy.single() else {
            continue;
        };

        let Some(star) = galaxy.star_in_system(system) else {
            continue;
        };

        let Some(universe_system) = universe_systems.system_mut(system) else {
            continue;
        };

        universe_system.add_item(star.location, Quat::IDENTITY, SystemItem::Star(star.star));
    }
}

// Errors can accumulate due to floating point imprecision - this avoids that
fn ensure_black_hole_never_moves(mut q_hole: Query<(&mut Location, &SpawnPos), With<BlackHole>>) {
    for (mut loc, spawn_pos) in q_hole.iter_mut() {
        if *loc != spawn_pos.0 {
            *loc = spawn_pos.0;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            generate_black_hole.in_set(SystemGenerationSet::BlackHole),
            load_black_holes_in_universe.in_set(FixedUpdateSet::Main),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        FixedUpdate,
        ensure_black_hole_never_moves
            .after(FixedUpdateSet::LocationSyncingPostPhysics)
            .before(FixedUpdateSet::PostLocationSyncingPostPhysics),
    )
    .add_systems(SAVING_SCHEDULE, on_save_black_hole.in_set(SavingSystemSet::DoSaving));
}
