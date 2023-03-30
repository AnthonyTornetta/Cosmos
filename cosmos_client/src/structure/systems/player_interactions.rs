use bevy::{
    prelude::{
        Added, App, Commands, Component, Entity, Input, IntoSystemConfigs, KeyCode, MouseButton,
        OnUpdate, Query, RemovedComponents, Res, ResMut, With,
    },
    reflect::{FromReflect, Reflect},
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_unreliable_messages::ClientUnreliableMessages, network_encoder, NettyChannel},
    structure::ship::pilot::Pilot,
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

#[derive(Component, Default, Reflect, FromReflect)]
struct HoveredSystem {
    system_index: usize,
    active: bool,
}

fn check_system_in_use(
    mut query: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    mut client: ResMut<RenetClient>,
) {
    if let Ok(mut hovered_system) = query.get_single_mut() {
        hovered_system.active =
            input_handler.check_pressed(CosmosInputs::UseSelectedSystem, &keys, &mouse);

        let active_system = if hovered_system.active {
            Some(hovered_system.system_index as u32)
        } else {
            None
        };

        client.send_message(
            NettyChannel::Unreliable.id(),
            network_encoder::serialize(&ClientUnreliableMessages::ShipActiveSystem {
                active_system,
            }),
        );
    }
}

fn check_became_pilot(
    mut commands: Commands,
    query: Query<Entity, (Added<Pilot>, With<LocalPlayer>)>,
) {
    for ent in query.iter() {
        commands.entity(ent).insert(HoveredSystem::default());
    }
}

fn swap_selected(
    mut query: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
) {
    if let Ok(mut hovered_system) = query.get_single_mut() {
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem1, &keys, &mouse) {
            hovered_system.system_index = 0;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem2, &keys, &mouse) {
            hovered_system.system_index = 1;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem3, &keys, &mouse) {
            hovered_system.system_index = 2;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem4, &keys, &mouse) {
            hovered_system.system_index = 3;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem5, &keys, &mouse) {
            hovered_system.system_index = 4;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem6, &keys, &mouse) {
            hovered_system.system_index = 5;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem7, &keys, &mouse) {
            hovered_system.system_index = 6;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem8, &keys, &mouse) {
            hovered_system.system_index = 7;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem9, &keys, &mouse) {
            hovered_system.system_index = 8;
        }
    }
}

fn check_removed_pilot(mut commands: Commands, mut removed: RemovedComponents<Pilot>) {
    for ent in removed.iter() {
        commands.entity(ent).remove::<HoveredSystem>();
    }
}

pub fn register(app: &mut App) {
    app.add_systems(
        (
            check_system_in_use,
            check_became_pilot,
            check_removed_pilot,
            swap_selected,
        )
            .in_set(OnUpdate(GameState::Playing)),
    )
    .register_type::<HoveredSystem>();
}
