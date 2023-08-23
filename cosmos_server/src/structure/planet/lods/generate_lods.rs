use std::time::Duration;

use bevy::{
    prelude::{in_state, App, Commands, Entity, IntoSystemConfigs, Query, Res, Update, With},
    time::common_conditions::on_timer,
};
use cosmos_core::{
    block::{Block, BlockFace},
    entities::player::Player,
    physics::location::Location,
    registry::Registry,
    structure::{lod::Lod, lod_chunk::LodChunk, planet::Planet, Structure},
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

fn generate_player_lods(
    mut commands: Commands,
    players: Query<(Entity, &Player, &Location)>,
    structures: Query<(Entity, &Structure, &Location), With<Planet>>,
    blocks: Res<Registry<Block>>,
) {
    for (player_entity, player, player_location) in players.iter() {
        for (ent, structure, location) in structures.iter() {
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

            commands.entity(ent).insert(PlayerLod {
                lod: all_stone_lod,
                player: player_entity,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        generate_player_lods
            .run_if(in_state(GameState::Playing))
            .run_if(on_timer(Duration::from_millis(1000))),
    );
}
