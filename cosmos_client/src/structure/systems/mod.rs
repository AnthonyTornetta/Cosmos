use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_unreliable_messages::ClientUnreliableMessages, NettyChannel},
    structure::{ship::pilot::Pilot, systems::SystemActive},
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

fn check_if_using_structure_system(
    query: Query<&Pilot, With<LocalPlayer>>,
    structure_query: Query<Entity>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    mut commands: Commands,
) {
    println!("Checked!");
    if let Ok(pilot) = query.get_single() {
        if let Ok(structure_ent) = structure_query.get(pilot.entity) {
            if input_handler.check_pressed(CosmosInputs::PlaceBlock, &keys, &mouse) {
                println!("Set system to active");
                commands.entity(structure_ent).insert(SystemActive);
            } else {
                println!("Set system to inactive");
                commands.entity(structure_ent).remove::<SystemActive>();
            }
        }
    }
}

fn send_structure_state(
    query: Query<&Pilot, With<LocalPlayer>>,
    structure_query: Query<&SystemActive>,
    mut client: ResMut<RenetClient>,
) {
    println!("Sending state??");
    if let Ok(pilot) = query.get_single() {
        println!("Sending state!!");

        client.send_message(
            NettyChannel::Unreliable.id(),
            bincode::serialize(&ClientUnreliableMessages::ShipStatus {
                use_system: structure_query.get(pilot.entity).is_ok(),
            })
            .unwrap(),
        );
    }
}

pub fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(check_if_using_structure_system)
            .with_system(send_structure_state),
    );
}
