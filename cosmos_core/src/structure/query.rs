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

pub struct MutBlockData<'q, 'w, 's, Q: QueryData> {
    data: QueryItem<'q, Q>,
    bs_params: Rc<RefCell<BlockDataSystemParams<'w, 's>>>,
    changed: bool,
    block: StructureBlock,
    structure_entity: Entity,
    data_entity: Entity,
}

impl<'q, 'w, 's, Q: QueryData> MutBlockData<'q, 'w, 's, Q> {
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
}

impl<'q, 'w, 's, Q: QueryData> Deref for MutBlockData<'q, 'w, 's, Q> {
    type Target = QueryItem<'q, Q>;

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

        self.bs_params.borrow_mut().ev_writer.send(BlockDataChangedEvent {
            block: self.block,
            block_data_entity: Some(self.data_entity),
            structure_entity: self.structure_entity,
        });
    }
}
