//! Represents the communications a laser cannon system sends

use bevy::{ecs::entity::Entity, prelude::Component};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::BlockCoordinate;

use super::Shop;

#[derive(Debug, Serialize, Deserialize, Component)]
pub enum ServerShopMessages {
    OpenShop {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        shop_data: Shop,
    },
    ShopContents {
        shop_block: BlockCoordinate,
        structure_entity: Entity,
        shop_data: Shop,
    },
}
