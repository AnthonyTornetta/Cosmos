use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{
            BlockBreakMessage, BlockInteractMessage, BlockMessagesSet, BlockPlaceMessage, InvalidBlockBreakMessageReason,
            InvalidBlockInteractMessageReason, InvalidBlockPlaceMessageReason,
        },
    },
    entities::player::Player,
    events::cancellable::{Cancellable, CancellableMessage},
    faction::{FactionId, Factions},
    netty::sync::events::{netty_event::NettyMessage, server_event::NettyMessageWriter},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{Structure, shared::MeltingDown, ship::Ship},
};

fn maybe_cancel_faction<E: NettyMessage>(
    event: &mut impl CancellableMessage,
    player: Entity,
    structure: Entity,
    q_factions: &Query<&FactionId>,
    q_melting_down: &Query<(), With<MeltingDown>>,
    q_player: &Query<&Player>,
    factions: &Factions,
    mw: &mut NettyMessageWriter<E>,
    e: E,
) {
    // we can break/place on melting down structures
    if q_melting_down.contains(structure) {
        return;
    }

    if let Some(broken_fac) = q_factions.get(structure).ok().and_then(|id| factions.from_id(id))
        && q_factions
            .get(player)
            .ok()
            .and_then(|id| factions.from_id(id))
            .map(|fac| fac.id() != broken_fac.id())
            .unwrap_or(true)
    {
        if let Ok(player) = q_player.get(player) {
            mw.write(e, player.client_id());
        }
        event.cancel();
    };
}

fn handle_placing_different_factions(
    mut evr_place: MessageMutator<Cancellable<BlockPlaceMessage>>,
    mut evr_break: MessageMutator<Cancellable<BlockBreakMessage>>,
    mut evr_interact: MessageMutator<Cancellable<BlockInteractMessage>>,
    q_melting_down: Query<(), With<MeltingDown>>,
    q_player: Query<&Player>,
    q_faction: Query<&FactionId>,
    factions: Res<Factions>,
    mut nevw_invalid_place: NettyMessageWriter<InvalidBlockPlaceMessageReason>,
    mut nevw_invalid_break: NettyMessageWriter<InvalidBlockBreakMessageReason>,
    mut nevw_invalid_interact: NettyMessageWriter<InvalidBlockInteractMessageReason>,
) {
    for place_event in evr_place.read() {
        let Cancellable::Active(place_event_data) = place_event else {
            continue;
        };

        let player = place_event_data.placer;
        let structure = place_event_data.block.structure();

        maybe_cancel_faction(
            place_event,
            player,
            structure,
            &q_faction,
            &q_melting_down,
            &q_player,
            &factions,
            &mut nevw_invalid_place,
            InvalidBlockPlaceMessageReason::DifferentFaction,
        );
    }

    for break_event in evr_break.read() {
        let Cancellable::Active(break_event_data) = break_event else {
            continue;
        };

        let player = break_event_data.breaker;
        let structure = break_event_data.block.structure();

        maybe_cancel_faction(
            break_event,
            player,
            structure,
            &q_faction,
            &q_melting_down,
            &q_player,
            &factions,
            &mut nevw_invalid_break,
            InvalidBlockBreakMessageReason::DifferentFaction,
        );
    }

    for interact_event in evr_interact.read() {
        let Cancellable::Active(interact_event_data) = interact_event else {
            continue;
        };

        let player = interact_event_data.interactor;
        let structure = interact_event_data
            .block
            .unwrap_or(interact_event_data.block_including_fluids)
            .structure();

        maybe_cancel_faction(
            interact_event,
            player,
            structure,
            &q_faction,
            &q_melting_down,
            &q_player,
            &factions,
            &mut nevw_invalid_interact,
            InvalidBlockInteractMessageReason::DifferentFaction,
        );
    }
}

fn handle_no_placing_cores(
    mut evr_place: MessageMutator<Cancellable<BlockPlaceMessage>>,
    mut evr_break: MessageMutator<Cancellable<BlockBreakMessage>>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    q_structure: Query<(&Structure, Option<&Ship>)>,
    mut nevw_invalid_break: NettyMessageWriter<InvalidBlockBreakMessageReason>,
) {
    for place_event in evr_place.read() {
        let Cancellable::Active(break_event_data) = place_event else {
            continue;
        };

        let block = blocks.from_numeric_id(break_event_data.block_id);

        if block.unlocalized_name() == "cosmos:ship_core" || block.unlocalized_name() == "cosmos:station_core" {
            if let Ok(player) = q_player.get(break_event_data.placer) {
                nevw_invalid_break.write(InvalidBlockBreakMessageReason::StructureCore, player.client_id());
            }
            place_event.cancel();
        }
    }

    for break_event in evr_break.read() {
        let Cancellable::Active(break_event_data) = break_event else {
            continue;
        };

        let block = blocks.from_numeric_id(break_event_data.broken_id);

        let Ok((structure, ship)) = q_structure.get(break_event_data.block.structure()) else {
            continue;
        };

        if ship
            .map(|s| s.ship_core_block_coords(structure) == break_event_data.block.coords())
            .unwrap_or(false)
            || block.unlocalized_name() == "cosmos:ship_core"
            || block.unlocalized_name() == "cosmos:station_core"
        {
            let mut itr = structure.all_blocks_iter(false);

            // ship core               some other block
            if itr.next().is_some() && itr.next().is_some() {
                // Do not allow player to mine ship core if another block exists on the ship

                if let Ok(player) = q_player.get(break_event_data.breaker) {
                    nevw_invalid_break.write(InvalidBlockBreakMessageReason::StructureCore, player.client_id());
                };
                break_event.cancel();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (handle_no_placing_cores, handle_placing_different_factions)
            .chain()
            .in_set(BlockMessagesSet::HandleBlockPlacementRules)
            .run_if(in_state(GameState::Playing)),
    );
}
