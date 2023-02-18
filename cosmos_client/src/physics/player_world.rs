use bevy::{
    prelude::{
        App, BuildChildren, Commands, Component, Entity, Parent, PbrBundle, Query,
        RemovedComponents, SystemSet, Transform, With, Without,
    },
    reflect::{FromReflect, Reflect},
};
use cosmos_core::physics::location::Location;

use crate::{netty::flags::LocalPlayer, state::game_state::GameState};

#[derive(Component, Debug, Reflect, FromReflect, Clone, Copy)]
pub struct PlayerWorld;

fn add_player_world(mut commands: Commands) {
    commands.spawn((
        PbrBundle {
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        },
        PlayerWorld,
        Location::default(),
    ));
}

fn monitor_removed_parent(
    removed: RemovedComponents<Parent>,
    world_query: Query<Entity, With<PlayerWorld>>,
    query: Query<(), (Without<LocalPlayer>, With<Location>)>, // With<Location> avoids targetting random UI elements
    mut commands: Commands,
) {
    for ent in removed.iter() {
        if let Ok(world) = world_query.get_single() {
            if query.contains(ent) {
                commands.entity(world).add_child(ent);
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<PlayerWorld>()
        .add_system_set(SystemSet::on_enter(GameState::LoadingWorld).with_system(add_player_world))
        .add_system(monitor_removed_parent);
}
