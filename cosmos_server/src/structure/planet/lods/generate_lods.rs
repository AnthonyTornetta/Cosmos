use std::time::Duration;

use bevy::{
    prelude::{
        in_state, warn, App, BuildChildren, Children, Commands, Component, DespawnRecursiveExt, Entity, Event, EventWriter,
        GlobalTransform, IntoSystemConfigs, Quat, Query, Res, Update, With,
    },
    time::common_conditions::on_timer,
};
use cosmos_core::{
    block::Block,
    entities::player::Player,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType, UnboundChunkCoordinate, UnboundCoordinateType},
        lod::Lod,
        lod_chunk::LodChunk,
        planet::Planet,
        Structure,
    },
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

#[derive(Debug)]
enum LodRequest {
    None,
    Same,
    Single,
    Multi(Box<[LodRequest; 8]>),
}

#[derive(Debug, Component)]
pub struct PlayerGeneratingLod {
    pub structure_entity: Entity,
    pub generating_lod: GeneratingLod,
    pub player_entity: Entity,
}

#[derive(Debug, Clone)]
/// Represents a reduced-detail version of a planet undergoing generation
pub enum GeneratingLod {
    /// No Lod here - this means there should be an actual chunk here
    None,
    /// Represents an LOD that needs generated
    NeedsGenerated,
    /// Represents an LOD that is currently being generated
    BeingGenerated,
    /// Represents a single chunk of blocks at any scale.
    DoneGenerating(Box<LodChunk>),
    /// Breaks a single cube into 8 sub-cubes.
    ///
    /// The indicies of each cube follow a clockwise direction starting on the bottom-left-back
    ///
    /// ```
    ///    +-----------+
    ///   /  5    6   /|
    ///  /  4    7   / |
    /// +-----------+  |
    /// |           |  |  
    /// |           |  +
    /// |   1    2  | /
    /// |  0    3   |/
    /// +-----------+
    /// ```
    Children(Box<[GeneratingLod; 8]>),
}

#[derive(Debug, Component)]
struct LodGenerationRequest {
    request: LodRequest,
    structure_entity: Entity,
    player_entity: Entity,
    // task: Task<Lod>,
}

#[derive(Event)]
pub struct GenerateLodRequest {
    pub starting_chunk: ChunkCoordinate,
    pub structure_entity: Entity,

    pub block_interval: CoordinateType,

    pub lod_chunk: LodChunk,
}

// fn generate_lod(mut query: Query<&mut PlayerGeneratingLod>, blocks: Res<Registry<Block>>) {
//     for mut generating_lod in query.iter_mut() {
//         recurse(&mut generating_lod.generating_lod, &blocks);
//     }
// }

fn check_done(generating_lod: &GeneratingLod) -> bool {
    match generating_lod {
        GeneratingLod::Children(children) => children.iter().all(check_done),
        GeneratingLod::None | GeneratingLod::DoneGenerating(_) => true,
        _ => false,
    }
}

fn recursively_create_lod(generated_lod: GeneratingLod) -> Lod {
    match generated_lod {
        GeneratingLod::Children(children) => {
            let [c0, c1, c2, c3, c4, c5, c6, c7] = *children;

            Lod::Children(Box::new([
                recursively_create_lod(c0),
                recursively_create_lod(c1),
                recursively_create_lod(c2),
                recursively_create_lod(c3),
                recursively_create_lod(c4),
                recursively_create_lod(c5),
                recursively_create_lod(c6),
                recursively_create_lod(c7),
            ]))
        }
        GeneratingLod::DoneGenerating(lod_chunk) => Lod::Single(lod_chunk),
        GeneratingLod::None => Lod::None,
        _ => {
            warn!("Invalid lod state: {generated_lod:?}");
            Lod::None
        }
    }
}

fn check_done_generating(mut commands: Commands, query: Query<(Entity, &PlayerGeneratingLod)>) {
    for (entity, player_generating_lod) in query.iter() {
        if check_done(&player_generating_lod.generating_lod) {
            commands.entity(entity).despawn_recursive();

            let actual_lod = recursively_create_lod(player_generating_lod.generating_lod.clone());

            commands.entity(player_generating_lod.structure_entity).with_children(|cmds| {
                cmds.spawn(PlayerLod {
                    lod: actual_lod,
                    player: player_generating_lod.player_entity,
                });
            });
        }
    }
}

fn fill_done_lod(lod: &Lod) -> GeneratingLod {
    match lod {
        Lod::None => GeneratingLod::None,
        Lod::Single(single) => GeneratingLod::DoneGenerating(single.clone()),
        Lod::Children(children) => GeneratingLod::Children(Box::new([
            fill_done_lod(&children[0]),
            fill_done_lod(&children[1]),
            fill_done_lod(&children[2]),
            fill_done_lod(&children[3]),
            fill_done_lod(&children[4]),
            fill_done_lod(&children[5]),
            fill_done_lod(&children[6]),
            fill_done_lod(&children[7]),
        ])),
    }
}

fn create_generating_lod(
    structure_entity: Entity,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<GenerateLodRequest>,
    request: &LodRequest,
    (min_block_range_inclusive, max_block_range_exclusive): (BlockCoordinate, BlockCoordinate),
    current_lod: Option<&Lod>,
) -> GeneratingLod {
    match request {
        LodRequest::Same => {
            let Some(current_lod) = current_lod else {
                panic!("Invalid current lod state - cannot be none!");
            };

            fill_done_lod(current_lod)
        }
        LodRequest::None => GeneratingLod::None,
        LodRequest::Single => {
            debug_assert!(
                max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.y - min_block_range_inclusive.y
                    && max_block_range_exclusive.x - min_block_range_inclusive.x
                        == max_block_range_exclusive.z - min_block_range_inclusive.z
            );
            let interval = (max_block_range_exclusive.x - min_block_range_inclusive.x + 1) / CHUNK_DIMENSIONS;

            event_writer.send(GenerateLodRequest {
                starting_chunk: ChunkCoordinate::for_block_coordinate(min_block_range_inclusive),
                structure_entity: structure_entity,
                block_interval: interval,
                lod_chunk: LodChunk::new(),
            });

            GeneratingLod::NeedsGenerated
        }
        LodRequest::Multi(child_requests) => {
            let (dx, dy, dz) = (
                (max_block_range_exclusive.x - min_block_range_inclusive.x) / 2,
                (max_block_range_exclusive.y - min_block_range_inclusive.y) / 2,
                (max_block_range_exclusive.z - min_block_range_inclusive.z) / 2,
            );

            let min = min_block_range_inclusive;
            let max = max_block_range_exclusive;

            let cur_lod_children = match current_lod {
                Some(Lod::Children(children)) => [
                    Some(&children[0]),
                    Some(&children[1]),
                    Some(&children[2]),
                    Some(&children[3]),
                    Some(&children[4]),
                    Some(&children[5]),
                    Some(&children[6]),
                    Some(&children[7]),
                ],
                _ => [None, None, None, None, None, None, None, None],
            };

            GeneratingLod::Children(Box::new([
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[0],
                    ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                    cur_lod_children[0],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[1],
                    ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                    cur_lod_children[1],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[2],
                    ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                    cur_lod_children[2],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[3],
                    ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                    cur_lod_children[3],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[4],
                    ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                    cur_lod_children[4],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[5],
                    ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                    cur_lod_children[5],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[6],
                    ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                    cur_lod_children[6],
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    event_writer,
                    &child_requests[7],
                    ((min.x + dx, min.y + dy, min.z).into(), (max.x, max.y, max.z - dz).into()),
                    cur_lod_children[7],
                ),
            ]))
        }
    }
}

fn poll_generating(
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
    mut event_writer: EventWriter<GenerateLodRequest>,
    structure_query: Query<(&Children, &Structure)>,
    query: Query<(Entity, &LodGenerationRequest)>,
    player_lod: Query<&PlayerLod>,
) {
    for (entity, lod_request) in query.iter() {
        let Ok((structure_children, structure)) = structure_query.get(lod_request.structure_entity) else {
            continue;
        };

        let current_lod = structure_children
            .iter()
            .flat_map(|&child_entity| player_lod.get(child_entity))
            .find(|p_lod| p_lod.player == lod_request.player_entity)
            .map(|p_lod| &p_lod.lod);

        let generating_lod = create_generating_lod(
            lod_request.structure_entity,
            &blocks,
            &mut event_writer,
            &lod_request.request,
            (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()),
            current_lod,
        );

        commands.spawn(PlayerGeneratingLod {
            structure_entity: lod_request.structure_entity,
            generating_lod,
            player_entity: lod_request.player_entity,
        });

        commands.entity(entity).despawn_recursive();
    }
}

fn create_lod_request(
    scale: CoordinateType,
    render_distance: CoordinateType,
    rel_coords: UnboundChunkCoordinate,
    first: bool,
    current_lod: Option<&Lod>,
) -> LodRequest {
    if scale == 0 {
        return match current_lod {
            Some(Lod::None) => LodRequest::Same,
            _ => LodRequest::None,
        };
    }

    let diameter = scale + render_distance - 1;

    let max_dist = diameter as UnboundCoordinateType;

    if !first && (rel_coords.x.abs() >= max_dist || rel_coords.y.abs() >= max_dist || rel_coords.z.abs() >= max_dist) {
        match current_lod {
            Some(Lod::Single(_)) => LodRequest::Same,
            _ => LodRequest::Single,
        }
    } else {
        let s4 = scale as UnboundCoordinateType / 4;

        LodRequest::Multi(Box::new([
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, -s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[0]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, -s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[1]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, -s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[2]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, -s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[3]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[4]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(-s4, s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[5]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, s4, s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[6]),
                    _ => None,
                },
            ),
            create_lod_request(
                scale / 2,
                render_distance,
                rel_coords - UnboundChunkCoordinate::new(s4, s4, -s4),
                false,
                match current_lod {
                    Some(Lod::Children(c)) => Some(&c[7]),
                    _ => None,
                },
            ),
        ]))
    }
}

fn generate_player_lods(
    mut commands: Commands,
    any_generating_lods: Query<(), With<LodGenerationRequest>>,
    players: Query<(Entity, &Player, &Location)>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform, Option<&Children>), With<Planet>>,
    current_lods: Query<&PlayerLod>,
) {
    if !any_generating_lods.is_empty() {
        return;
    }

    for (player_entity, player, player_location) in players.iter() {
        let render_distance = 4;

        for (structure_ent, structure, structure_location, g_trans, children) in structures.iter() {
            let Structure::Dynamic(ds) = structure else {
                panic!("Planet was a non-dynamic!!!");
            };

            let inv_rotation = Quat::from_affine3(&g_trans.affine().inverse());
            let rel_coords = inv_rotation.mul_vec3(structure_location.relative_coords_to(player_location));

            let scale = ds.chunk_dimensions();

            let rel_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ds.relative_coords_to_local_coords(
                rel_coords.x,
                rel_coords.y,
                rel_coords.z,
            ));

            let middle_chunk = UnboundChunkCoordinate::new(
                scale as UnboundCoordinateType / 2,
                scale as UnboundCoordinateType / 2,
                scale as UnboundCoordinateType / 2,
            );

            let current_lod = children
                .map(|c| {
                    c.iter()
                        .flat_map(|&child_entity| current_lods.get(child_entity))
                        .find(|p_lod| p_lod.player == player_entity)
                        .map(|p_lod| &p_lod.lod)
                })
                .unwrap_or(None);

            let request = create_lod_request(scale, render_distance, rel_coords - middle_chunk, true, current_lod);

            let request_entity = commands
                .spawn(LodGenerationRequest {
                    player_entity,
                    structure_entity: structure_ent,
                    request,
                })
                .id();
            commands.entity(structure_ent).add_child(request_entity);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            generate_player_lods
                .run_if(in_state(GameState::Playing))
                .run_if(on_timer(Duration::from_millis(10000))),
            (poll_generating, check_done_generating).run_if(in_state(GameState::Playing)),
        ),
    )
    .add_event::<GenerateLodRequest>();
}
