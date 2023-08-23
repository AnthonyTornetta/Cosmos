use bevy::prelude::{in_state, Added, App, Commands, Component, Entity, IntoSystemConfigs, Query, Res, States, Update};
use serde::{Deserialize, Serialize};

use crate::{
    block::{Block, BlockFace},
    registry::Registry,
};

use super::{lod_chunk::LodChunk, planet::Planet, Structure};

#[derive(Serialize, Deserialize, Component, Debug)]
pub enum Lod {
    None,
    Single(Box<LodChunk>),
    Children(Box<[Lod; 8]>),
}

fn add_lod_to_planet(blocks: Res<Registry<Block>>, mut commands: Commands, query: Query<(Entity, &Structure), Added<Planet>>) {
    for (ent, structure) in query.iter() {
        let Structure::Dynamic(ds) = structure else {
            panic!("Planet was a non-dynamic!!!");
        };

        let mut chunk1 = LodChunk::new(ds.dimensions() / 2);
        chunk1.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk2 = LodChunk::new(ds.dimensions() / 2);
        chunk2.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk3 = LodChunk::new(ds.dimensions() / 2);
        chunk3.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk4 = LodChunk::new(ds.dimensions() / 2);
        chunk4.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);

        let mut chunk5 = LodChunk::new(ds.dimensions() / 2);
        chunk5.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk6 = LodChunk::new(ds.dimensions() / 2);
        chunk6.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk7 = LodChunk::new(ds.dimensions() / 2);
        chunk7.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
        let mut chunk8 = LodChunk::new(ds.dimensions() / 2);
        chunk8.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);

        let all_stone_lod = Lod::Children(Box::new([
            Lod::Single(Box::new(chunk1)),
            Lod::Single(Box::new(chunk2)),
            Lod::Single(Box::new(chunk3)),
            Lod::Single(Box::new(chunk4)),
            Lod::Single(Box::new(chunk5)),
            Lod::Single(Box::new(chunk6)),
            Lod::Single(Box::new(chunk7)),
            Lod::Single(Box::new(chunk8)),
        ]));

        // let all_stone_lod = Lod::Children(Box::new([
        //     Lod::None,
        //     Lod::None,
        //     Lod::None,
        //     Lod::None,
        //     Lod::None,
        //     Lod::None,
        //     Lod::None,
        //     Lod::Children(Box::new([
        //         Lod::Single(Box::new(chunk1)),
        //         Lod::Single(Box::new(chunk2)),
        //         Lod::Single(Box::new(chunk3)),
        //         Lod::Single(Box::new(chunk4)),
        //         Lod::Single(Box::new(chunk5)),
        //         Lod::Single(Box::new(chunk6)),
        //         Lod::Single(Box::new(chunk7)),
        //         Lod::Single(Box::new(chunk8)),
        //     ])),
        // ]));

        commands.entity(ent).insert(all_stone_lod);
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, playing_state: T) {
    app.add_systems(Update, add_lod_to_planet.run_if(in_state(playing_state)));
}
