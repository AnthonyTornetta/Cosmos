use std::time::Duration;

use bevy::{
    prelude::{in_state, App, Commands, Component, DespawnRecursiveExt, Entity, IntoSystemConfigs, Query, Res, Update, With},
    tasks::Task,
    time::common_conditions::on_timer,
};
use cosmos_core::{
    block::{Block, BlockFace},
    entities::player::Player,
    physics::location::Location,
    registry::Registry,
    structure::{coordinates::BlockCoordinate, lod::Lod, lod_chunk::LodChunk, planet::Planet, Structure},
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

#[derive(Debug)]
enum LodRequest {
    Single,
    Multi(Box<[LodRequest; 8]>),
}

#[derive(Debug, Component)]
struct LodGenerationRequest {
    request: LodRequest,
    structure_entity: Entity,
    player_entity: Entity,
    // task: Task<()>,
}

fn create_lod(
    blocks: &Registry<Block>,

    request: &LodRequest,
    (min_block_range_inclusive, max_block_range_exclusive): (BlockCoordinate, BlockCoordinate),
) -> Lod {
    match request {
        LodRequest::Single => {
            let mut chunk = LodChunk::new();
            chunk.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            Lod::Single(Box::new(chunk))
        }
        LodRequest::Multi(child_requests) => {
            let (dx, dy, dz) = (
                (max_block_range_exclusive.x - min_block_range_inclusive.x) / 2,
                (max_block_range_exclusive.y - min_block_range_inclusive.y) / 2,
                (max_block_range_exclusive.z - min_block_range_inclusive.z) / 2,
            );

            let min = min_block_range_inclusive;
            let max = max_block_range_exclusive;

            Lod::Children(Box::new([
                create_lod(
                    blocks,
                    &child_requests[0],
                    ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[1],
                    ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[2],
                    ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[3],
                    ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[4],
                    ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[5],
                    ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[6],
                    ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                ),
                create_lod(
                    blocks,
                    &child_requests[7],
                    ((min.x + dx, min.y + dy, min.z).into(), (max.x, max.y, max.z - dz).into()),
                ),
            ]))
        }
    }
}

fn poll_generating(
    mut commands: Commands,
    blocks: Res<Registry<Block>>,

    structure_query: Query<&Structure>,
    query: Query<(Entity, &LodGenerationRequest)>,
) {
    for (entity, lod_request) in query.iter() {
        let Ok(structure) = structure_query.get(lod_request.structure_entity) else {
            continue;
        };

        let lod = create_lod(
            &blocks,
            &lod_request.request,
            (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()),
        );

        commands.entity(lod_request.structure_entity).insert(PlayerLod {
            lod,
            player: lod_request.player_entity,
        });

        commands.entity(entity).despawn_recursive();
    }
}

fn generate_player_lods(
    mut commands: Commands,
    any_generating_lods: Query<(), With<LodGenerationRequest>>,
    players: Query<(Entity, &Player, &Location)>,
    structures: Query<(Entity, &Structure, &Location), With<Planet>>,
) {
    if !any_generating_lods.is_empty() {
        return;
    }

    for (player_entity, player, player_location) in players.iter() {
        for (structure_ent, structure, structure_location) in structures.iter() {
            let Structure::Dynamic(ds) = structure else {
                panic!("Planet was a non-dynamic!!!");
            };

            let request = LodRequest::Multi(Box::new([
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
                LodRequest::Single,
            ]));

            let lod_request = LodGenerationRequest {
                player_entity,
                structure_entity: structure_ent,
                request,
            };

            // let mut chunk1 = LodChunk::new();
            // chunk1.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk2 = LodChunk::new();
            // chunk2.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk3 = LodChunk::new();
            // chunk3.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk4 = LodChunk::new();
            // chunk4.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);

            // let mut chunk5 = LodChunk::new();
            // chunk5.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk6 = LodChunk::new();
            // chunk6.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk7 = LodChunk::new();
            // chunk7.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);
            // let mut chunk8 = LodChunk::new();
            // chunk8.fill(blocks.from_id("cosmos:stone").expect("Missing stone!"), BlockFace::Top);

            // let all_stone_lod = Lod::Children(Box::new([
            //     Lod::Single(Box::new(chunk1)),
            //     Lod::Single(Box::new(chunk2)),
            //     Lod::Single(Box::new(chunk3)),
            //     Lod::Single(Box::new(chunk4)),
            //     Lod::Single(Box::new(chunk5)),
            //     Lod::Single(Box::new(chunk6)),
            //     Lod::Single(Box::new(chunk7)),
            //     Lod::Single(Box::new(chunk8)),
            // ]));

            commands.spawn(lod_request);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            generate_player_lods
                .run_if(in_state(GameState::Playing))
                .run_if(on_timer(Duration::from_millis(1000))),
            poll_generating.run_if(in_state(GameState::Playing)),
        ),
    );
}
