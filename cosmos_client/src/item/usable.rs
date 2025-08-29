use bevy::prelude::*;
use cosmos_core::{
    item::usable::PlayerRequestUseHeldItemEvent,
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    state::GameState,
    structure::ship::pilot::Pilot,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    interactions::block_interactions::LookingAt,
};

fn on_use_item(
    inputs: InputChecker,
    mut nevw_use_item: NettyEventWriter<PlayerRequestUseHeldItemEvent>,
    q_player: Query<&LookingAt, (With<LocalPlayer>, Without<Pilot>)>,
) {
    if !inputs.check_just_pressed(CosmosInputs::UseHeldItem) {
        return;
    }
    let Ok(looking_at) = q_player.single() else {
        return;
    };

    nevw_use_item.write(PlayerRequestUseHeldItemEvent {
        looking_at_block: looking_at.looking_at_block.map(|x| x.block),
        looking_at_any: looking_at.looking_at_any.map(|x| x.block),
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_use_item.run_if(in_state(GameState::Playing)));
}
