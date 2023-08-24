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
        for (structure_ent, structure, structure_location) in structures.iter() {
            let Structure::Dynamic(ds) = structure else {
                panic!("Planet was a non-dynamic!!!");
            };

            let mut chunk1 = LodChunk::new();
            chunk1.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk2 = LodChunk::new();
            chunk2.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk3 = LodChunk::new();
            chunk3.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk4 = LodChunk::new();
            chunk4.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);

            let mut chunk5 = LodChunk::new();
            chunk5.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk6 = LodChunk::new();
            chunk6.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk7 = LodChunk::new();
            chunk7.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            let mut chunk8 = LodChunk::new();
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

            commands.entity(structure_ent).insert(PlayerLod {
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
