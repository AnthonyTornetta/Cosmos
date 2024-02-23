use bevy::{
    app::App,
    ecs::{
        component::Component,
        query::With,
        system::{Commands, Query},
    },
};
use cosmos_core::{entities::player::Player, physics::location::Location};

#[derive(Component)]
pub struct Pirate;

fn spawn_pirates(mut commands: Commands, q_players: Query<&Location, With<Player>>, q_pirate_ships: Query<&Location, With<Pirate>>) {}

pub(super) fn register(app: &mut App) {}
