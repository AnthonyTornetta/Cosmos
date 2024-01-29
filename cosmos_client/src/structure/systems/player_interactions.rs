use bevy::{
    prelude::{in_state, Added, App, Commands, Component, Entity, IntoSystemConfigs, Query, RemovedComponents, ResMut, Update, With},
    reflect::Reflect,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{client_unreliable_messages::ClientUnreliableMessages, cosmos_encoder, NettyChannelClient},
    structure::ship::pilot::Pilot,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

#[derive(Component, Default, Reflect)]
struct HoveredSystem {
    system_index: usize,
    active: bool,
}

fn check_system_in_use(
    mut query: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>,
    input_handler: InputChecker,
    mut client: ResMut<RenetClient>,
) {
    let Ok(mut hovered_system) = query.get_single_mut() else {
        return;
    };

    hovered_system.active = input_handler.check_pressed(CosmosInputs::UseSelectedSystem);

    let active_system = if hovered_system.active {
        Some(hovered_system.system_index as u32)
    } else {
        None
    };

    client.send_message(
        NettyChannelClient::Unreliable,
        cosmos_encoder::serialize(&ClientUnreliableMessages::ShipActiveSystem { active_system }),
    );
}

fn check_became_pilot(mut commands: Commands, query: Query<Entity, (Added<Pilot>, With<LocalPlayer>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(HoveredSystem::default());
    }
}

fn swap_selected(mut query: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>, input_handler: InputChecker) {
    if let Ok(mut hovered_system) = query.get_single_mut() {
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem1) {
            hovered_system.system_index = 0;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem2) {
            hovered_system.system_index = 1;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem3) {
            hovered_system.system_index = 2;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem4) {
            hovered_system.system_index = 3;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem5) {
            hovered_system.system_index = 4;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem6) {
            hovered_system.system_index = 5;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem7) {
            hovered_system.system_index = 6;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem8) {
            hovered_system.system_index = 7;
        }
        if input_handler.check_just_pressed(CosmosInputs::SelectSystem9) {
            hovered_system.system_index = 8;
        }
    }
}

fn check_removed_pilot(mut commands: Commands, mut removed: RemovedComponents<Pilot>) {
    for ent in removed.read() {
        if let Some(mut ecmds) = commands.get_entity(ent) {
            ecmds.remove::<HoveredSystem>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (check_system_in_use, check_became_pilot, check_removed_pilot, swap_selected).run_if(in_state(GameState::Playing)),
    )
    .register_type::<HoveredSystem>();
}
