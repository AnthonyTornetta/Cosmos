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

use crate::events::block_events::{BlockDataChangedMessage, BlockDataSystemParams};

use super::structure_block::StructureBlock;

/// A wrapper around a mutable block data query result. This is used to send out block data changed
/// events when a change is detected, preventing unexpected errors.
pub struct MutBlockData<'q, 'w, 's, Q: QueryData> {
    data: QueryItem<'q, 'q, Q>,
    bs_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
    changed: bool,
    block: StructureBlock,
    data_entity: Entity,
}

impl<'q, 'w, 's, Q: QueryData> MutBlockData<'q, 'w, 's, Q> {
    /// Creates a wrapper around a mutable block data query result.
    ///
    /// When this item goes out of scope, if a mutable reference has been gotten, an event will be sent indicating
    /// this block's data has been changed.
    pub fn new(
        data: QueryItem<'q, 'q, Q>,
        bs_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
        block: StructureBlock,
        data_entity: Entity,
    ) -> Self {
        Self {
            changed: false,
            data,
            bs_params,
            block,
            data_entity,
        }
    }

    /// Returns a mutable reference to the data WITHOUT triggering change detection
    pub fn bypass_change_detection(&mut self) -> &mut QueryItem<'q, 'q, Q> {
        &mut self.data
    }
}

impl<'q, 'w, 's, Q: QueryData> Deref for MutBlockData<'q, 'w, 's, Q> {
    type Target = QueryItem<'q, 'q, Q>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'q, 'w, 's, Q: QueryData> DerefMut for MutBlockData<'q, 'w, 's, Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.changed = true;

        &mut self.data
    }
}

impl<'q, 'w, 's, Q: QueryData> Drop for MutBlockData<'q, 'w, 's, Q> {
    fn drop(&mut self) {
        if !self.changed {
            return;
        }

        self.bs_params.borrow_mut().ev_writer.write(BlockDataChangedMessage {
            block: self.block,
            block_data_entity: Some(self.data_entity),
        });
    }
}
