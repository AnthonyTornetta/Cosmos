use bevy::prelude::{in_state, Added, App, Commands, Component, Entity, IntoSystemConfigs, Query, Res, States, Update};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::Registry,
};

use super::{dynamic_structure::DynamicStructure, lod_chunk::LodChunk, planet::Planet, Structure};

#[derive(Serialize, Deserialize, Component, Debug)]
pub enum Lod {
    None,
    Single(Box<LodChunk>),
    Children(Box<[LodChunk; 4]>),
}

fn add_lod_to_planet(blocks: Res<Registry<Block>>, mut commands: Commands, query: Query<(Entity, &Structure), Added<Planet>>) {
    for (ent, structure) in query.iter() {
        let Structure::Dynamic(ds) = structure else {
            panic!("Planet was a non-dynamic!!!");
        };

        let mut chunk = LodChunk::new(ds.dimensions());
        chunk.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let all_stone_lod = Lod::Single(Box::new(chunk));
        commands.entity(ent).insert(all_stone_lod);
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_systems(Update, add_lod_to_planet.run_if(in_state(playing_state)));
}
