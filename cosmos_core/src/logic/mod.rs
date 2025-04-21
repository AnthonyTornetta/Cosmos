//! The game's logic system: for wires, logic gates, etc.

use std::{collections::VecDeque, time::Duration};

use bevy::{
    app::{App, Update},
    prelude::{
        Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, Query, Res, ResMut, Resource, States, SystemSet,
        With, Without, in_state,
    },
    reflect::Reflect,
    time::common_conditions::on_timer,
    utils::{HashMap, HashSet},
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{
        Block,
        block_direction::{ALL_BLOCK_DIRECTIONS, BlockDirection},
        block_events::BlockEventsSet,
        block_face::BlockFace,
        data::BlockData,
    },
    events::block_events::{BlockChangedEvent, BlockDataChangedEvent, BlockDataSystemParams},
    netty::{
        sync::{IdentifiableComponent, SyncableComponent, sync_component},
        system_sets::NetworkingSystemsSet,
    },
    registry::{Registry, create_registry, identifiable::Identifiable},
    structure::{Structure, coordinates::BlockCoordinate, loading::StructureLoadingSet, structure_block::StructureBlock},
};

#[derive(Component, Clone, Copy, Reflect, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
/// The logic signal this block is holding.
///
/// NOTE: Each block might interact with this data slightly differently.
///
/// Usually, a block with an output port will calculate this value the frame before outputting it and store it here.
pub struct BlockLogicData(pub i32);

impl BlockLogicData {
    /// For Boolean applications. 0 is "off" or "false", anything else is "on" or "true".
    pub fn on(&self) -> bool {
        self.0 != 0
    }
}

impl IdentifiableComponent for BlockLogicData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:block_logic_data"
    }
}

impl SyncableComponent for BlockLogicData {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<BlockLogicData>(app);

    app.register_type::<BlockLogicData>();
}
