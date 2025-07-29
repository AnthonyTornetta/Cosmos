use std::fs;

use bevy::prelude::*;
use cosmos_core::entities::player::Player;

use crate::commands::{Operator, Operators, cosmos_command_handler::ProcessCommandsSet};

fn load_operators(mut commands: Commands) {
    let operators = fs::read_to_string("operators.json")
        .map(|x| serde_json::from_str::<Operators>(&x).unwrap_or_else(|e| panic!("Failed to parse operators.json - {e:?}")))
        .unwrap_or_default();

    commands.insert_resource(operators);
}

fn on_update_operators(mut commands: Commands, q_players: Query<(Entity, &Player)>, operators: Res<Operators>) {
    if let Err(e) = fs::write("operators.json", serde_json::to_string_pretty(operators.as_ref()).unwrap()) {
        error!("Failed to write operators.toml! {e:?}");
    }

    for (ent, player) in q_players.iter() {
        if operators.is_operator(player.client_id()) {
            commands.entity(ent).insert(Operator);
        } else {
            commands.entity(ent).remove::<Operator>();
        }
    }
}

fn on_update_players(mut commands: Commands, q_players: Query<(Entity, &Player), Changed<Player>>, operators: Res<Operators>) {
    for (ent, player) in q_players.iter() {
        if operators.is_operator(player.client_id()) {
            commands.entity(ent).insert(Operator);
        } else {
            commands.entity(ent).remove::<Operator>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, load_operators).add_systems(
        FixedUpdate,
        (on_update_operators, on_update_players)
            .chain()
            .before(ProcessCommandsSet::ParseCommands),
    );
}
