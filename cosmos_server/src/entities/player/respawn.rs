use bevy::prelude::*;
use cosmos_core::{entities::EntityId, prelude::BlockCoordinate};
use serde::{Deserialize, Serialize};

#[derive(Component, Reflect, Serialize, Deserialize)]
pub struct RespawnBlock {
    block_coord: BlockCoordinate,
    structure_id: EntityId,
}

pub(super) fn register(app: &mut App) {
    app.register_type::<RespawnBlock>();
}
