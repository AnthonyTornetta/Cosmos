//! Utilities for querying things within structures

use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use bevy::{
    ecs::query::{QueryData, QueryItem},
    prelude::Entity,
};

use crate::events::block_events::{BlockDataChangedEvent, BlockDataSystemParams};

use super::structure_block::StructureBlock;

/// A wrapper around a mutable block data query result. This is used to send out block data changed
/// events when a change is detected, preventing unexpected errors.
pub struct MutBlockData<'q, 'w, 's, Q: QueryData> {
    data: QueryItem<'q, Q>,
    bs_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
    changed: bool,
    block: StructureBlock,
    structure_entity: Entity,
    data_entity: Entity,
}

impl<'q, 'w, 's, Q: QueryData> MutBlockData<'q, 'w, 's, Q> {
    /// Creates a wrapper around a mutable block data query result.
    ///
    /// When this item goes out of scope, if a mutable reference has been gotten, an event will be sent indicating
    /// this block's data has been changed.
    pub fn new(
        data: QueryItem<'q, Q>,
        bs_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
        block: StructureBlock,
        structure_entity: Entity,
        data_entity: Entity,
    ) -> Self {
        Self {
            changed: false,
            data,
            bs_params,
            block,
            data_entity,
            structure_entity,
        }
    }

    /// Returns a mutable reference to the data WITHOUT triggering change detection
    pub fn bypass_change_detection(&mut self) -> &mut QueryItem<'q, Q> {
        &mut self.data
    }
}

impl<'q, Q: QueryData> Deref for MutBlockData<'q, '_, '_, Q> {
    type Target = QueryItem<'q, Q>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<Q: QueryData> DerefMut for MutBlockData<'_, '_, '_, Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;

        &mut self.data
    }
}

impl<Q: QueryData> Drop for MutBlockData<'_, '_, '_, Q> {
    fn drop(&mut self) {
        if !self.changed {
            return;
        }

        self.bs_params.borrow_mut().ev_writer.send(BlockDataChangedEvent {
            block: self.block,
            block_data_entity: Some(self.data_entity),
            structure_entity: self.structure_entity,
        });
    }
}
