use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockBreakMessage, BlockInteractMessage, BlockMessagesSet, BlockPlaceMessage, InvalidBlockPlaceMessageReason},
    },
    entities::player::Player,
    events::cancellable::CancellableMessage,
    netty::sync::events::server_event::NettyMessageWriter,
    registry::{Registry, identifiable::Identifiable},
    structure::{Structure, shared::MeltingDown},
};

fn handle_placing_different_factions(
    mut evr_place: MessageMutator<BlockPlaceMessage>,
    mut evr_break: MessageMutator<BlockBreakMessage>,
    mut evr_interact: MessageMutator<BlockInteractMessage>,
    blocks: Res<Registry<Block>>,
    // q_faction: Query<&FactionId>,
    // factions: Res<Factions>,
    q_melting_down: Query<(), With<MeltingDown>>,
    q_player: Query<&Player>,
    mut nevw_invalid_place: NettyMessageWriter<InvalidBlockPlaceMessageReason>,
) {
    for place_event in evr_place.read() {
        let BlockPlaceMessage::Message(place_event_data) = place_event else {
            continue;
        };

        let block = blocks.from_numeric_id(place_event_data.block_id);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            if let Ok(player) = q_player.get(place_event_data.placer) {
                nevw_invalid_place.write(InvalidBlockPlaceMessageReason::CoreBlock, player.client_id());
            }
            place_event.cancel();
        }
    }
}

fn handle_no_placing_cores(
    mut evr_place: MessageMutator<BlockPlaceMessage>,
    mut evr_break: MessageMutator<BlockBreakMessage>,
    blocks: Res<Registry<Block>>,
    // q_faction: Query<&FactionId>,
    // factions: Res<Factions>,
    q_player: Query<&Player>,
    mut nevw_invalid_place: NettyMessageWriter<InvalidBlockPlaceMessageReason>,
) {
    for place_event in evr_place.read() {
        let BlockPlaceMessage::Message(place_event_data) = place_event else {
            continue;
        };

        let block = blocks.from_numeric_id(place_event_data.block_id);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            if let Ok(player) = q_player.get(place_event_data.placer) {
                nevw_invalid_place.write(InvalidBlockPlaceMessageReason::CoreBlock, player.client_id());
            }
            place_event.cancel();
        }
    }

    for break_event in evr_break.read() {
        let BlockBreakMessage::Message(break_event_data) = break_event else {
            continue;
        };

        let block = blocks.from_numeric_id(break_event_data.block_id);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            if let Ok(player) = q_player.get(place_event_data.placer) {
                nevw_invalid_place.write(InvalidBlockPlaceMessageReason::CoreBlock, player.client_id());
            }
            place_event.cancel();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_no_placing_cores.in_set(BlockMessagesSet::HandleBlockPlacementRules),
    );
}
