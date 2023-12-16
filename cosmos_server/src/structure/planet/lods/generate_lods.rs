use std::{marker::PhantomData, mem::swap};

use bevy::{
    prelude::{
        debug, in_state, App, BuildChildren, Children, Commands, Component, Deref, DerefMut, Entity, GlobalTransform, IntoSystemConfigs,
        Parent, Quat, Query, Res, ResMut, Resource, Update, With,
    },
    tasks::Task,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundChunkCoordinate, UnboundCoordinateType},
        lod::{Lod, ReadOnlyLod},
        lod_chunk::LodChunk,
        planet::Planet,
        shared::DespawnWithStructure,
        Structure,
    },
};
use futures_lite::future;

use crate::state::GameState;

use super::player_lod::PlayerLod;

#[derive(Debug)]
enum LodRequest {
    Same,
    Single,
    Multi(Box<[LodRequest; 8]>),
}

#[derive(Debug, Component, Clone)]
pub struct LodNeedsGeneratedForPlayer {
    pub structure_entity: Entity,
    pub generating_lod: GeneratingLod,
    pub player_entity: Entity,
    pub current_lod: Option<ReadOnlyLod>,
}

#[derive(Debug, Component, Clone)]
pub struct DoneGeneratingLod {
    /// Represents `LodNetworkMessage::SetLod` but is pre-serialized to save time when sending this to players
    pub lod_delta: Vec<u8>,
    pub new_lod: Lod,
    pub cloned_new_lod: Lod,
}

#[derive(Debug)]
pub struct AsyncGeneratingLod<T> {
    player_entity: Entity,
    structure_entity: Entity,
    task: Task<DoneGeneratingLod>,
    _phantom: PhantomData<T>,
}

impl<T> AsyncGeneratingLod<T> {
    pub fn new(player_entity: Entity, structure_entity: Entity, task: Task<DoneGeneratingLod>) -> Self {
        Self {
            _phantom: PhantomData,
            player_entity,
            structure_entity,
            task,
        }
    }
}

#[derive(Debug, Resource, Deref, DerefMut, Default)]
pub(crate) struct GeneratingLods<T>(pub Vec<AsyncGeneratingLod<T>>);

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
pub(crate) struct LodGenerationRequest {
    request: LodRequest,
    structure_entity: Entity,
    player_entity: Entity,
    current_lod: Option<ReadOnlyLod>,
}

pub(crate) fn check_done_generating_lods<T: Component + Default>(
    mut commands: Commands,
    mut lod_query: Query<(Entity, &mut PlayerLod, &Parent)>,
    mut generating_lods: ResMut<GeneratingLods<T>>,
) {
    let mut todo = Vec::with_capacity(generating_lods.capacity());

    swap(&mut todo, &mut generating_lods.0);

    for mut task in todo {
        if let Some(done_generating_lod) = future::block_on(future::poll_once(&mut task.task)) {
            let lod = done_generating_lod.new_lod;
            let read_only_lod = ReadOnlyLod::from(done_generating_lod.cloned_new_lod);
            let lod_delta = done_generating_lod.lod_delta;

            if let Some((_, mut player_lod, _)) = lod_query
                .iter_mut()
                .find(|(_, player_lod, parent)| player_lod.player == task.player_entity && parent.get() == task.structure_entity)
            {
                player_lod.lod = lod;
                player_lod.deltas.push(lod_delta);
                player_lod.read_only_lod = read_only_lod;
            } else if let Some(mut ecmds) = commands.get_entity(task.structure_entity) {
                ecmds.with_children(|cmds| {
                    cmds.spawn((
                        PlayerLod {
                            lod,
                            deltas: vec![lod_delta],
                            player: task.player_entity,
                            read_only_lod,
                        },
                        DespawnWithStructure,
                    ));
                });
            }
        } else {
            generating_lods.push(task);
        }
    }
}

fn create_generating_lod(
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
                    &child_requests[0],
                    ((min.x, min.y, min.z).into(), (max.x - dx, max.y - dy, max.z - dz).into()),
                ),
                create_generating_lod(
                    &child_requests[1],
                    ((min.x, min.y, min.z + dz).into(), (max.x - dx, max.y - dy, max.z).into()),
                ),
                create_generating_lod(
                    &child_requests[2],
                    ((min.x + dx, min.y, min.z + dz).into(), (max.x, max.y - dy, max.z).into()),
                ),
                create_generating_lod(
                    &child_requests[3],
                    ((min.x + dx, min.y, min.z).into(), (max.x, max.y - dy, max.z - dz).into()),
                ),
                create_generating_lod(
                    &child_requests[4],
                    ((min.x, min.y + dy, min.z).into(), (max.x - dx, max.y, max.z - dz).into()),
                ),
                create_generating_lod(
                    &child_requests[5],
                    ((min.x, min.y + dy, min.z + dz).into(), (max.x - dx, max.y, max.z).into()),
                ),
                create_generating_lod(
                    &child_requests[6],
                    ((min.x + dx, min.y + dy, min.z + dz).into(), (max.x, max.y, max.z).into()),
                ),
                create_generating_lod(
                    &child_requests[7],
                    ((min.x + dx, min.y + dy, min.z).into(), (max.x, max.y, max.z - dz).into()),
                ),
            ]))
        }
    }
}

pub(crate) fn start_generating_lods(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    query: Query<(Entity, &LodGenerationRequest)>,
) {
    for (entity, lod_request) in query.iter() {
        let Ok(structure) = structure_query.get(lod_request.structure_entity) else {
            continue;
        };

        let generating_lod = create_generating_lod(&lod_request.request, (BlockCoordinate::new(0, 0, 0), structure.block_dimensions()));

        debug!("Starting to generate lod for {:?}", lod_request.structure_entity);

        commands
            .entity(entity)
            .remove::<LodGenerationRequest>()
            .insert(LodNeedsGeneratedForPlayer {
                structure_entity: lod_request.structure_entity,
                generating_lod,
                player_entity: lod_request.player_entity,
                current_lod: lod_request.current_lod.clone(),
            });
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

pub(crate) fn generate_player_lods<T: Component + Default>(
    mut commands: Commands,
    any_generation_requests: Query<Entity, (With<LodGenerationRequest>, With<T>)>,
    needs_generated_lods: Query<&LodNeedsGeneratedForPlayer>,
    players: Query<(Entity, &Location), With<Player>>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform, Option<&Children>), (With<Planet>, With<T>)>,
    current_lods: Query<&PlayerLod>,
    currently_generating_lods: Res<GeneratingLods<T>>,
) {
    if !any_generation_requests.is_empty() {
        return;
    }

    for (player_entity, player_location) in players.iter() {
        let render_distance = 4;

        for (structure_ent, structure, structure_location, g_trans, children) in structures.iter() {
            if needs_generated_lods.iter().any(|needs_generated_lod| {
                needs_generated_lod.player_entity == player_entity && needs_generated_lod.structure_entity == structure_ent
            }) {
                continue;
            }

            if currently_generating_lods
                .iter()
                .any(|generating_lod| generating_lod.player_entity == player_entity || generating_lod.structure_entity == structure_ent)
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
                        .map(|p_lod| (&p_lod.lod, p_lod.read_only_lod.clone()))
                })
                .unwrap_or(None);

            let request = create_lod_request(
                scale,
                render_distance,
                rel_coords - middle_chunk,
                true,
                current_lod.as_ref().map(|x| x.0),
            );

            // Same lod, don't generate
            if matches!(request, LodRequest::Same) {
                continue;
            }

            debug!("Requesting new lod generation for {structure_ent:?}");

            let request_entity = commands
                .spawn((
                    LodGenerationRequest {
                        player_entity,
                        structure_entity: structure_ent,
                        request,
                        current_lod: current_lod.map(|x| x.1),
                    },
                    T::default(),
                ))
                .id();
            commands.entity(structure_ent).add_child(request_entity);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (start_generating_lods.run_if(in_state(GameState::Playing)),));
}
