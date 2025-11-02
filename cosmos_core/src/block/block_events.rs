//! Contains the various types of block events

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    structure::structure_block::StructureBlock,
};

use super::block_rotation::BlockRotation;

/// This is sent whenever a player breaks a block
#[derive(Debug, Message)]
pub struct BlockBreakMessage {
    /// The player breaking the block
    pub breaker: Entity,
    /// The block broken with
    pub block: StructureBlock,
    /// The block that was broken's id
    pub broken_id: u16,
}

#[derive(Debug, Message, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be broken, but was rejected by the server. Sent to the client
pub enum InvalidBlockBreakMessageReason {
    /// The structure this block was broken on does not allow breaking by this player
    DifferentFaction,
    /// The structure's core block (ship core or station core) must be the last block a player
    /// breaks.
    StructureCore,
}

impl IdentifiableMessage for InvalidBlockBreakMessageReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_break_event_reason"
    }
}

impl NettyMessage for InvalidBlockBreakMessageReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

/// This is sent whenever a player interacts with a block
#[derive(Debug, Message)]
pub struct BlockInteractMessage {
    /// The block that was interacted with by the player
    pub block: Option<StructureBlock>,
    /// Includes blocks normally ignored by most interaction checks
    pub block_including_fluids: StructureBlock,
    /// The player that interacted with the block
    pub interactor: Entity,
    /// If this is true, the player was crouching while interacting with this block
    ///
    /// If the block being interacted with has two modes of interaction, this should be used to trigger
    /// the second mode.
    pub alternate: bool,
}

#[derive(Debug, Message, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be interacted with, but was rejected by the server. Sent to the client
pub enum InvalidBlockInteractMessageReason {
    /// The structure this block was interacted with does not allow interactions by this player
    DifferentFaction,
}

impl IdentifiableMessage for InvalidBlockInteractMessageReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_interact_event_reason"
    }
}

impl NettyMessage for InvalidBlockInteractMessageReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

#[derive(Debug, Message)]
/// Sent when a block is trying to be placed.
///
/// Used to request block placements (such as from the player)
pub enum BlockPlaceMessage {
    /// This event has been cancelled and should no longer be processed - the block placement is no longer happening
    Cancelled,
    /// This event is a valid block place event that should be processed and verified
    Message(BlockPlaceMessageData),
}

/// This is sent whenever a player places a block
#[derive(Debug, Message, Clone, Copy)]
pub struct BlockPlaceMessageData {
    /// Where the block is placed
    pub structure_block: StructureBlock,
    /// The placed block's id
    pub block_id: u16,
    /// The block's rotation
    pub block_up: BlockRotation,
    /// The inventory slot this block came from
    pub inventory_slot: usize,
    /// The player who placed this block
    pub placer: Entity,
}

#[derive(Debug, Message, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be placed, but was rejected by the server. Sent to the client
pub enum InvalidBlockPlaceMessageReason {
    /// The structure this block was placed on does not allow placements by this player
    DifferentFaction,
}

impl IdentifiableMessage for InvalidBlockPlaceMessageReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_place_event_reason"
    }
}

impl NettyMessage for InvalidBlockPlaceMessageReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The event set used for processing block events
pub enum BlockMessagesSet {
    /// Block place events for this frame should be done in or before this set
    SendMessagesForThisFrame,
    /// In this set, you should put systems that can be cancel/remove events.
    PreProcessMessages,
    /// Block updates are sent here
    SendBlockUpdateMessages,
    /// All block events processing happens here - during this set the block is NOT guarenteed to be placed or removed yet or have its data created
    ///
    /// Please note that at this point, the only event sent may be the [`BlockPlaceMessage`] - not the resulting [`BlockChangedMessage`].
    /// The [`BlockChangedMessage`] is only sent once the block is inserted into the structure (which happens during this set).
    ProcessMessagesPrePlacement,
    /// The structure updates blocks based on the [`BlockPlaceMessage`] and send [`BlockChangedMessage`].
    ChangeBlocks,
    /// If your event processing relies on the block being placed, run it in this set. The data still is not guarenteed to be present.
    ProcessMessages,
    /// For systems that need information set in the [`BlockMessagesSet::ProcessMessages`] stage. Block data should be present.
    PostProcessMessages,
    /// Put systems that send events you want read the next frame here.
    SendMessagesForNextFrame,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            BlockMessagesSet::SendMessagesForThisFrame,
            BlockMessagesSet::PreProcessMessages,
            BlockMessagesSet::SendBlockUpdateMessages,
            BlockMessagesSet::ProcessMessagesPrePlacement,
            BlockMessagesSet::ChangeBlocks,
            BlockMessagesSet::ProcessMessages,
            BlockMessagesSet::PostProcessMessages,
            BlockMessagesSet::SendMessagesForNextFrame,
        )
            .chain(), // .after(StructureLoadingSet::StructureLoaded),
    )
    .add_netty_event::<InvalidBlockBreakMessageReason>()
    .add_netty_event::<InvalidBlockPlaceMessageReason>()
    .add_netty_event::<InvalidBlockInteractMessageReason>();
}
