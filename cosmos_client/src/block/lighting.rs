//! Handles all the blocks with lighting in the game

use bevy::{
    prelude::{App, Color, OnExit, Res, ResMut},
    reflect::Reflect,
    utils::warn,
};
use cosmos_core::{
    block::Block,
    registry::{self, identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::state::game_state::GameState;

#[derive(Debug, Reflect, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
/// If a block has light, it will have a block light property
pub struct BlockLightProperties {
    /// The color of that light
    pub color: Color,
    /// How intense it should be in lumens,
    ///
    /// See https://docs.rs/bevy/latest/bevy/pbr/struct.PointLight.html for a table of valus.
    pub intensity: f32,
    /// How far this light will reach
    pub range: f32,
    /// Ignored for now due to performance issues. Shadows are currently always disabled.
    pub shadows_disabled: bool,
}

#[derive(Debug, Clone, Reflect, Default, Serialize, Deserialize)]
/// This links up a block to its block light properties
pub struct BlockLighting {
    /// The properties this block has
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

fn register_light(lighting: BlockLightProperties, registry: &mut Registry<BlockLighting>, blocks: &Registry<Block>, name: &str) {
    if let Some(block) = blocks.from_id(name) {
        registry.register(BlockLighting {
            properties: lighting,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        });
    } else {
        warn(Err(format!("[Block Lighting] Missing block {name}")));
    }
}

fn register_all_lights(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockLighting>>) {
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

pub(super) fn register(app: &mut App) {
    registry::create_registry::<BlockLighting>(app);

    app.add_systems(OnExit(GameState::Loading), register_all_lights);
}
