use bevy::prelude::{
    in_state, warn, App, BuildChildren, Children, Commands, Component, DespawnRecursiveExt, Entity, GlobalTransform, IntoSystemConfigs,
    Quat, Query, Res, Update, With,
};
use cosmos_core::{
    block::Block,
    entities::player::Player,
    physics::location::Location,
    registry::Registry,
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundChunkCoordinate, UnboundCoordinateType},
        lod::{Lod, LodDelta},
        lod_chunk::LodChunk,
        planet::Planet,
        Structure,
    },
};

use crate::state::GameState;

use super::player_lod::PlayerLod;

#[derive(Debug)]
enum LodRequest {
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
    /// Represents an LOD that needs generated
    NeedsGenerated,
    /// Represents an LOD that is currently being generated
    BeingGenerated,
    /// Represents a single chunk of blocks at any scale.
    DoneGenerating(Box<LodChunk>),
    /// Represents no change required
    Same,
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

fn check_done(generating_lod: &GeneratingLod) -> bool {
    match generating_lod {
        GeneratingLod::Children(children) => children.iter().all(check_done),
        GeneratingLod::DoneGenerating(_) | GeneratingLod::Same => true,
        _ => false,
    }
}

fn recursively_create_lod_delta(generated_lod: GeneratingLod) -> LodDelta {
    match generated_lod {
        GeneratingLod::Same => LodDelta::NoChange,
        GeneratingLod::Children(children) => {
            let [c0, c1, c2, c3, c4, c5, c6, c7] = *children;

            LodDelta::Children(Box::new([
                recursively_create_lod_delta(c0),
                recursively_create_lod_delta(c1),
                recursively_create_lod_delta(c2),
                recursively_create_lod_delta(c3),
                recursively_create_lod_delta(c4),
                recursively_create_lod_delta(c5),
                recursively_create_lod_delta(c6),
                recursively_create_lod_delta(c7),
            ]))
        }
        GeneratingLod::DoneGenerating(lod_chunk) => LodDelta::Single(lod_chunk),
        _ => {
            warn!("Invalid lod state: {generated_lod:?}");
            LodDelta::None
        }
    }
}

fn check_done_generating(
    mut commands: Commands,
    children_query: Query<&Children>,
    mut lod_query: Query<(Entity, &mut PlayerLod)>,
    query: Query<(Entity, &PlayerGeneratingLod)>,
) {
    for (entity, player_generating_lod) in query.iter() {
        if check_done(&player_generating_lod.generating_lod) {
            commands.entity(entity).despawn_recursive();

            let current_lod = children_query
                .get(player_generating_lod.structure_entity)
                .map(|children| {
                    children
                        .iter()
                        .flat_map(|&child_entity| lod_query.get(child_entity))
                        .find(|&(_, player_lod)| player_lod.player == player_generating_lod.player_entity)
                        .map(|(entity, _)| entity)
                })
                .unwrap_or(None)
                .map(|e| lod_query.get_mut(e).map(|(_, player_lod)| player_lod));

            let lod_delta = recursively_create_lod_delta(player_generating_lod.generating_lod.clone());

            let cloned_delta = lod_delta.clone();

            if let Some(Ok(mut current_lod)) = current_lod {
                cloned_delta.apply_changes(&mut current_lod.lod);
                current_lod.deltas.push(lod_delta);
            } else {
                commands.get_entity(player_generating_lod.structure_entity).map(|mut ecmds| {
                    ecmds.with_children(|cmds| {
                        cmds.spawn(PlayerLod {
                            lod: cloned_delta.create_lod(),
                            deltas: vec![lod_delta],
                            player: player_generating_lod.player_entity,
                        });
                    });
                });
            }
        }
    }
}

fn create_generating_lod(
    structure_entity: Entity,
    blocks: &Registry<Block>,
    request: &LodRequest,
    (min_block_range_inclusive, max_block_range_exclusive): (BlockCoordinate, BlockCoordinate),
) -> GeneratingLod {
    match request {
        LodRequest::Same => GeneratingLod::Same,
        LodRequest::Single => {
            debug_assert!(
                max_block_range_exclusive.x - min_block_range_inclusive.x == max_block_range_exclusive.y - min_block_range_inclusive.y
                    && max_block_range_exclusive.x - min_block_range_inclusive.x
                        == max_block_range_exclusive.z - min_block_range_inclusive.z
            );

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

            GeneratingLod::Children(Box::new([
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[0],
                    ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[1],
                    ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[2],
                    ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[3],
                    ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[4],
                    ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[5],
                    ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                ),
                create_generating_lod(
                    structure_entity,
                    blocks,
                    &child_requests[6],
                    ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                ),
                create_generating_lod(
                    structure_entity,
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

        let generating_lod = create_generating_lod(
            lod_request.structure_entity,
            &blocks,
            &lod_request.request,
            (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()),
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
    if scale == 1 {
        return match current_lod {
            Some(Lod::Single(_, _)) => LodRequest::Same,
            _ => LodRequest::Single,
        };
    }

    let diameter = scale + render_distance - 1;

    let max_dist = diameter as UnboundCoordinateType;

    if !first && (rel_coords.x.abs() >= max_dist || rel_coords.y.abs() >= max_dist || rel_coords.z.abs() >= max_dist) {
        match current_lod {
            Some(Lod::Single(_, _)) => LodRequest::Same,
            _ => LodRequest::Single,
        }
    } else {
        let s4 = scale as UnboundCoordinateType / 4;

        let children = [
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
        ];

        if children.iter().all(|x| matches!(x, LodRequest::Same)) {
            LodRequest::Same
        } else {
            LodRequest::Multi(Box::new(children))
        }
    }
}

fn generate_player_lods(
    mut commands: Commands,
    any_generation_requests: Query<(), With<LodGenerationRequest>>,
    generating_lods: Query<&PlayerGeneratingLod>,
    players: Query<(Entity, &Location), With<Player>>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform, Option<&Children>), With<Planet>>,
    current_lods: Query<&PlayerLod>,
) {
    if !any_generation_requests.is_empty() {
        return;
    }

    for (player_entity, player_location) in players.iter() {
        let render_distance = 4;

        for (structure_ent, structure, structure_location, g_trans, children) in structures.iter() {
            if generating_lods
                .iter()
                .any(|generating_lod| generating_lod.player_entity == player_entity && generating_lod.structure_entity == structure_ent)
            {
                continue;
            }

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

            // Same lod, don't generate
            if matches!(request, LodRequest::Same) {
                continue;
            }

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
            generate_player_lods.run_if(in_state(GameState::Playing)),
            (poll_generating, check_done_generating).run_if(in_state(GameState::Playing)),
        ),
    );
}
