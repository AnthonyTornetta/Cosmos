use bevy::{
    prelude::{App, Color, Res, ResMut, SystemSet},
    reflect::{FromReflect, Reflect},
};
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::state::game_state::GameState;

#[derive(Debug, Reflect, FromReflect, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct BlockLightProperties {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub shadows_disabled: bool,
}

#[derive(Debug, Reflect, FromReflect, Default, Serialize, Deserialize)]
pub struct BlockLighting {
    pub properties: BlockLightProperties,

    id: u16,
    unlocalized_name: String,
}

impl Identifiable for BlockLighting {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

fn register_light(
    lighting: BlockLightProperties,
    registry: &mut Registry<BlockLighting>,
    blocks: &Registry<Block>,
    name: &str,
) {
    if let Some(block) = blocks.from_id(name) {
        registry.register(BlockLighting {
            properties: lighting,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        });
    } else {
        println!("[Block Lighting] Missing block {name}");
    }
}

fn register_all_lights(
    blocks: Res<Registry<Block>>,
    mut registry: ResMut<Registry<BlockLighting>>,
) {
    register_light(
        BlockLightProperties {
            color: Color::WHITE,
            intensity: 500.0,
            range: 12.0,
            ..Default::default()
        },
        &mut registry,
        &blocks,
        "cosmos:light",
    );

    register_light(
        BlockLightProperties {
            color: Color::rgb(81.0 / 255.0, 143.0 / 255.0, 225.0 / 255.0),
            intensity: 100.0,
            range: 6.0,
            ..Default::default()
        },
        &mut registry,
        &blocks,
        "cosmos:ship_core",
    );
}

pub(crate) fn register(app: &mut App) {
    registry::create_registry::<BlockLighting>(app);

    app.add_system_set(SystemSet::on_exit(GameState::Loading).with_system(register_all_lights));
}
