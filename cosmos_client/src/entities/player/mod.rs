//! Contains systems and components for the player

use bevy::{color::palettes::css, prelude::*};
use bevy_rapier3d::prelude::{ActiveEvents, CoefficientCombineRule, Collider, Friction, LockedAxes, ReadMassProperties, RigidBody};
use cosmos_core::{ecs::sets::FixedUpdateSet, entities::player::Player, persistence::LoadingDistance, state::GameState};

pub mod death;
pub mod player_movement;
pub mod render_distance;

fn on_add_player(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    q_player: Query<(Entity, &Player), Added<Player>>,
) {
    for (ent, player) in q_player.iter() {
        commands.entity(ent).insert((
            Mesh3d(meshes.add(Capsule3d::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: css::GREEN.into(),
                ..Default::default()
            })),
            Collider::capsule_y(0.65, 0.25),
            LockedAxes::ROTATION_LOCKED,
            Name::new(format!("Player ({})", player.name())),
            RigidBody::Dynamic,
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            LoadingDistance::new(1, 2),
            ReadMassProperties::default(),
            ActiveEvents::COLLISION_EVENTS,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        on_add_player
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    );

    render_distance::register(app);
    player_movement::register(app);
    death::register(app);
}
