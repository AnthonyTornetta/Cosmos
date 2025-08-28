//! Contains the various types of block events

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    structure::structure_block::StructureBlock,
};

use super::block_rotation::BlockRotation;

/// This is sent whenever a player breaks a block
#[derive(Debug, Event)]
pub struct BlockBreakEvent {
    /// The player breaking the block
    pub breaker: Entity,
    /// The block broken with
    pub block: StructureBlock,
    /// The block that was broken's id
    pub broken_id: u16,
}

#[derive(Debug, Event, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be broken, but was rejected by the server. Sent to the client
pub enum InvalidBlockBreakEventReason {
    /// The structure this block was broken on does not allow breaking by this player
    DifferentFaction,
    /// The structure's core block (ship core or station core) must be the last block a player
    /// breaks.
    StructureCore,
}

impl IdentifiableEvent for InvalidBlockBreakEventReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_break_event_reason"
    }
}

impl NettyEvent for InvalidBlockBreakEventReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

/// This is sent whenever a player interacts with a block
#[derive(Debug, Event)]
pub struct BlockInteractEvent {
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

#[derive(Debug, Event, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be interacted with, but was rejected by the server. Sent to the client
pub enum InvalidBlockInteractEventReason {
    /// The structure this block was interacted with does not allow interactions by this player
    DifferentFaction,
}

impl IdentifiableEvent for InvalidBlockInteractEventReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_interact_event_reason"
    }
}

impl NettyEvent for InvalidBlockInteractEventReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

#[derive(Debug, Event)]
/// Sent when a block is trying to be placed.
///
/// Used to request block placements (such as from the player)
pub enum BlockPlaceEvent {
    /// This event has been cancelled and should no longer be processed - the block placement is no longer happening
    Cancelled,
    /// This event is a valid block place event that should be processed and verified
    Event(BlockPlaceEventData),
}

/// This is sent whenever a player places a block
#[derive(Debug, Event, Clone, Copy)]
pub struct BlockPlaceEventData {
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

#[derive(Debug, Event, Serialize, Deserialize, Clone, Copy)]
/// A block was attempted to be placed, but was rejected by the server. Sent to the client
pub enum InvalidBlockPlaceEventReason {
    /// The structure this block was placed on does not allow placements by this player
    DifferentFaction,
}

impl IdentifiableEvent for InvalidBlockPlaceEventReason {
    fn unlocalized_name() -> &'static str {
        "cosmos:invalid_block_place_event_reason"
    }
}

impl NettyEvent for InvalidBlockPlaceEventReason {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The event set used for processing block events
pub enum BlockEventsSet {
    /// Block place events for this frame should be done in or before this set
    SendEventsForThisFrame,
    /// In this set, you should put systems that can be cancel/remove events.
    PreProcessEvents,
    /// Block updates are sent here
    SendBlockUpdateEvents,
    /// All block events processing happens here - during this set the block is NOT guarenteed to be placed or removed yet or have its data created
    ///
    /// Please note that at this point, the only event sent may be the [`BlockPlaceEvent`] - not the resulting [`BlockChangedEvent`].
    /// The [`BlockChangedEvent`] is only sent once the block is inserted into the structure (which happens during this set).
    ProcessEventsPrePlacement,
    /// The structure updates blocks based on the [`BlockPlaceEvent`] and send [`BlockChangedEvent`].
    ChangeBlocks,
    /// If your event processing relies on the block being placed, run it in this set. The data still is not guarenteed to be present.
    ProcessEvents,
    /// For systems that need information set in the [`BlockEventsSet::ProcessEvents`] stage. Block data should be present.
    PostProcessEvents,
    /// Put systems that send events you want read the next frame here.
    SendEventsForNextFrame,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            BlockEventsSet::SendEventsForThisFrame,
            BlockEventsSet::PreProcessEvents,
            BlockEventsSet::SendBlockUpdateEvents,
            BlockEventsSet::ProcessEventsPrePlacement,
            BlockEventsSet::ChangeBlocks,
            BlockEventsSet::ProcessEvents,
            BlockEventsSet::PostProcessEvents,
            BlockEventsSet::SendEventsForNextFrame,
        )
            .chain(), // .after(StructureLoadingSet::StructureLoaded),
    )
    .add_netty_event::<InvalidBlockBreakEventReason>()
    .add_netty_event::<InvalidBlockPlaceEventReason>()
    .add_netty_event::<InvalidBlockInteractEventReason>();
}
