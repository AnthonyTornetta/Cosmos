//! Used to represent how much damage a block can take before it breaks

use bevy::prelude::{App, OnExit, Res, ResMut, States};

use crate::registry::{self, identifiable::Identifiable, Registry};

use super::Block;

#[derive(Debug)]
/// Used to represent how much damage a block can take before it breaks
pub struct BlockHardness {
    id: u16,
    unlocalized_name: String,

    // Air: 0, Leaves: 1, Grass/Dirt: 10, Stone: 50, Hull: 100,
    hardness: f32,
}

impl BlockHardness {
    /// Creates a new hardness value for that block.
    ///
    /// This still needs to be registered!
    pub fn new(block: &Block, hardness: f32) -> BlockHardness {
        Self {
            id: 0,
            unlocalized_name: block.unlocalized_name.to_owned(),
            hardness,
        }
    }

    /// Gets the hardness value
    pub fn hardness(&self) -> f32 {
        self.hardness
    }
}

impl Identifiable for BlockHardness {
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

fn register_hardness(registry: &mut Registry<BlockHardness>, value: f32, blocks: &Registry<Block>, name: &str) {
    if let Some(block) = blocks.from_id(name) {
        registry.register(BlockHardness::new(block, value));
    } else {
        println!("[Block Hardness] Missing block {name}");
    }
}

fn register_block_hardness(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<BlockHardness>>) {
    register_hardness(&mut registry, 0.0, &blocks, "cosmos:air");
    register_hardness(&mut registry, 10.0, &blocks, "cosmos:grass");
    register_hardness(&mut registry, 10.0, &blocks, "cosmos:dirt");
    register_hardness(&mut registry, 50.0, &blocks, "cosmos:stone");
    register_hardness(&mut registry, 50.0, &blocks, "cosmos:molten_stone");

    register_hardness(&mut registry, 30.0, &blocks, "cosmos:log");

    register_hardness(&mut registry, 1.0, &blocks, "cosmos:cherry_leaf");

    register_hardness(&mut registry, 30.0, &blocks, "cosmos:redwood_log");
    register_hardness(&mut registry, 1.0, &blocks, "cosmos:redwood_leaf");
    register_hardness(&mut registry, 10.0, &blocks, "cosmos:cheese");

    register_hardness(&mut registry, 100.0, &blocks, "cosmos:ship_core");
    register_hardness(&mut registry, 20.0, &blocks, "cosmos:energy_cell");
    register_hardness(&mut registry, 20.0, &blocks, "cosmos:reactor");
    register_hardness(&mut registry, 20.0, &blocks, "cosmos:laser_cannon");
    register_hardness(&mut registry, 20.0, &blocks, "cosmos:thruster");
    register_hardness(&mut registry, 20.0, &blocks, "cosmos:light");

    register_hardness(&mut registry, 100.0, &blocks, "cosmos:ship_hull");
    register_hardness(&mut registry, 100.0, &blocks, "cosmos:glass");
}

fn sanity_check(blocks: Res<Registry<Block>>, hardness: Res<Registry<BlockHardness>>) {
    for block in blocks.iter() {
        if hardness.from_id(block.unlocalized_name()).is_none() {
            eprintln!("!!! WARNING !!! Missing block hardness value for {}", block.unlocalized_name());
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, loading_state: T, post_loading_state: T) {
    registry::create_registry::<BlockHardness>(app);

    app.add_systems(OnExit(loading_state), register_block_hardness);
    app.add_systems(OnExit(post_loading_state), sanity_check);
}
