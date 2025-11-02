use crate::asset::asset_loading::BlockTextureIndex;
use crate::asset::materials::{AddMaterialMessage, BlockMaterialMapping, MaterialDefinition, MaterialType, RemoveAllMaterialsMessage};
use crate::block::lighting::{BlockLightProperties, BlockLighting};
use crate::rendering::structure_renderer::{BlockRenderingModes, StructureRenderingSet};
use crate::rendering::{CosmosMeshBuilder, ReadOnlyBlockMeshRegistry};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::MeshAabb;
use bevy::render::primitives::Aabb;
use bevy::tasks::AsyncComputeTaskPool;
use cosmos_core::block::Block;
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::physics::location::SECTOR_DIMENSIONS;
use cosmos_core::prelude::ChunkBlockCoordinate;
use cosmos_core::registry::ReadOnlyRegistry;
use cosmos_core::registry::many_to_one::ReadOnlyManyToOneRegistry;
use cosmos_core::structure::chunk::{CHUNK_DIMENSIONS, ChunkEntity};
use cosmos_core::structure::coordinates::UnboundChunkCoordinate;
use cosmos_core::structure::{ChunkNeighbors, Structure};
use futures_lite::future;

use super::chunk_renderer::{ChunkNeedsCustomBlocksRendered, ChunkRenderer, RenderingChunk, RenderingChunks};
use super::neighbor_checking::ChunkRenderingChecker;
use super::{ChunkMeshes, ChunkNeedsRendered, ChunkRenderResult, LightEntry, LightsHolder};

fn poll_rendering_chunks(
    mut commands: Commands,
    mut rendering_chunks: ResMut<RenderingChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    q_mesh: Query<&Mesh3d>,
    q_lights: Query<&LightsHolder>,
    q_chunk_meshes: Query<&ChunkMeshes>,
    q_chunk_entity: Query<&ChunkEntity>,
    mut evw_add_material_event: MessageWriter<AddMaterialMessage>,
    mut evw_remove_all_materials: MessageWriter<RemoveAllMaterialsMessage>,
    mut evw_chunk_needs_custom_blocks_rerendered: MessageWriter<ChunkNeedsCustomBlocksRendered>,
) {
    let mut todo = Vec::with_capacity(rendering_chunks.capacity());

    std::mem::swap(&mut rendering_chunks.0, &mut todo);

    // Reverse to iterate from most recent to least recent
    todo.reverse();
    let mut events_to_send = Vec::new();

    for mut rendering_chunk in todo {
        let Some(rendered_chunk) = future::block_on(future::poll_once(&mut rendering_chunk.0)) else {
            rendering_chunks.push(rendering_chunk);
            continue;
        };

        let (entity, mut chunk_mesh) = (rendered_chunk.chunk_entity, rendered_chunk.mesh);

        if commands.get_entity(entity).is_err() {
            // Chunk may have been despawned during its rendering
            continue;
        }

        if let Ok(chunk_entity) = q_chunk_entity.get(entity) {
            // This should be sent even if there are no custom blocks, because the chunk may have had
            // custom blocks in the past that need their rendering info cleaned up
            let ev = ChunkNeedsCustomBlocksRendered {
                block_ids: rendered_chunk.custom_blocks,
                chunk_coordinate: chunk_entity.chunk_location,
                mesh_entity_parent: entity,
                structure_entity: chunk_entity.structure_entity,
            };

            if events_to_send.contains(&ev) {
                // We have already rendered a more up-to-date version of this chunk, so stop doing anything for this outdated version.
                continue;
            }

            events_to_send.push(ev);
        }

        // The old entities can be reused to gain some performance (needs to be measured to see if there's an actual difference)
        let mut old_mesh_entities = Vec::new();

        if let Ok(chunk_meshes_component) = q_chunk_meshes.get(entity) {
            for ent in chunk_meshes_component.0.iter() {
                if let Ok(old_mesh_handle) = q_mesh.get(*ent) {
                    meshes.remove(old_mesh_handle);
                }

                old_mesh_entities.push(*ent);
            }
        }

        let mut entities_to_add = vec![];

        let new_lights = create_lighting_data(&q_lights, entity, chunk_mesh.lights, &mut commands, &mut entities_to_add);

        // meshes

        // The first mesh a chunk has will be on the chunk entity instead of child entities,
        // so clear that out first.
        commands.entity(entity).remove::<Mesh3d>();
        evw_remove_all_materials.write(RemoveAllMaterialsMessage { entity });

        let mut chunk_meshes_component = ChunkMeshes::default();

        while chunk_mesh.mesh_materials.len() > 1 {
            let mesh_material = chunk_mesh.mesh_materials.pop().expect("Checked above");

            let mesh = meshes.add(mesh_material.mesh);

            let s = (CHUNK_DIMENSIONS / 2) as f32;

            let ent = commands
                .spawn((
                    Mesh3d(mesh),
                    Transform::default(),
                    Visibility::default(),
                    // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                    Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                ))
                .id();

            entities_to_add.push(ent);

            evw_add_material_event.write(AddMaterialMessage {
                entity: ent,
                add_material_id: mesh_material.material_id,
                texture_dimensions_index: mesh_material.texture_dimensions_index,
                material_type: MaterialType::Normal,
            });

            chunk_meshes_component.0.push(ent);
        }

        if !chunk_mesh.mesh_materials.is_empty() {
            // To avoid making too many entities (and tanking performance), if only one mesh
            // is present, just stick the mesh info onto the chunk itself.

            let mesh_material = chunk_mesh.mesh_materials.pop().expect("This has one element in it");

            let aabb = mesh_material.mesh.compute_aabb();
            let mesh = meshes.add(mesh_material.mesh);

            commands.entity(entity).insert((
                Mesh3d(mesh),
                // mesh_material.material_id,
                // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                aabb.unwrap_or_default(),
            ));

            evw_add_material_event.write(AddMaterialMessage {
                entity,
                add_material_id: mesh_material.material_id,
                texture_dimensions_index: mesh_material.texture_dimensions_index,
                material_type: MaterialType::Normal,
            });
        }

        // Any leftover entities are useless now, so kill them
        for mesh in old_mesh_entities {
            commands.entity(mesh).despawn();
        }

        let mut entity_commands = commands.entity(entity);

        for ent in entities_to_add {
            entity_commands.add_child(ent);
        }

        entity_commands.insert(new_lights).insert(chunk_meshes_component);
    }

    // Undo the reverse above
    rendering_chunks.0.reverse();

    evw_chunk_needs_custom_blocks_rerendered.write_batch(events_to_send);
}

fn create_lighting_data(
    q_lights: &Query<&LightsHolder>,
    entity: Entity,
    rendered_lights: HashMap<ChunkBlockCoordinate, BlockLightProperties>,
    commands: &mut Commands,
    entities_to_add: &mut Vec<Entity>,
) -> LightsHolder {
    let mut new_lights = LightsHolder::default();

    if let Ok(lights) = q_lights.get(entity) {
        for light in lights.lights.iter() {
            let mut light = *light;
            light.valid = false;
            new_lights.lights.push(light);
        }
    }

    if !rendered_lights.is_empty() {
        for light in rendered_lights {
            let (block_light_coord, properties) = light;

            let mut found = false;
            for light in new_lights.lights.iter_mut() {
                if light.position.x == block_light_coord.x
                    && light.position.y == block_light_coord.y
                    && light.position.z == block_light_coord.z
                {
                    if light.light == properties {
                        light.valid = true;
                        found = true;
                    }
                    break;
                }
            }

            if !found {
                let light_entity = commands
                    .spawn((
                        PointLight {
                            color: properties.color,
                            intensity: properties.intensity,
                            range: properties.range,
                            radius: 1.0,
                            // Shadows kill all performance
                            shadows_enabled: false, // !properties.shadows_disabled,
                            ..Default::default()
                        },
                        Transform::from_xyz(
                            block_light_coord.x as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                            block_light_coord.y as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                            block_light_coord.z as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                        ),
                    ))
                    .id();

                new_lights.lights.push(LightEntry {
                    entity: light_entity,
                    light: properties,
                    position: block_light_coord,
                    valid: true,
                });
                entities_to_add.push(light_entity);
            }
        }
    }

    for light in new_lights.lights.iter().filter(|x| !x.valid) {
        commands.entity(light.entity).despawn();
    }

    new_lights.lights.retain(|x| x.valid);

    new_lights
}

/// Performance hot spot
fn monitor_needs_rendered_system(
    mut commands: Commands,
    structure_query: Query<&Structure>,
    blocks: Res<ReadOnlyRegistry<Block>>,
    materials: Res<ReadOnlyManyToOneRegistry<Block, BlockMaterialMapping>>,
    meshes_registry: Res<ReadOnlyBlockMeshRegistry>,
    lighting: Res<ReadOnlyRegistry<BlockLighting>>,
    block_textures: Res<ReadOnlyRegistry<BlockTextureIndex>>,
    mut rendering_chunks: ResMut<RenderingChunks>,
    local_player: Query<&GlobalTransform, With<LocalPlayer>>,
    chunks_need_rendered: Query<(Entity, &ChunkEntity, &GlobalTransform), With<ChunkNeedsRendered>>,
    materials_registry: Res<ReadOnlyRegistry<MaterialDefinition>>,
    block_rendering_mode: Res<BlockRenderingModes>,
) {
    let Ok(local_transform) = local_player.single() else {
        return;
    };

    for (entity, ce, _) in chunks_need_rendered
        .iter()
        .map(|(x, y, transform)| (x, y, transform.translation().distance_squared(local_transform.translation())))
        // Only render chunks that are within a reasonable viewing distance
        .filter(|(_, _, distance_sqrd)| *distance_sqrd < SECTOR_DIMENSIONS * SECTOR_DIMENSIONS)
    {
        let async_task_pool = AsyncComputeTaskPool::get();

        let Ok(structure) = structure_query.get(ce.structure_entity) else {
            continue;
        };

        let coords = ce.chunk_location;

        // I assure you officer, cloning 7 chunks to render 1 is very necessary
        //
        // please someone fix this when they feel inspired

        let Some(chunk) = structure.chunk_at(coords).cloned() else {
            continue;
        };

        let unbound = UnboundChunkCoordinate::from(coords);

        let pos_x = structure.chunk_at_unbound(unbound.pos_x()).cloned();
        let neg_x = structure.chunk_at_unbound(unbound.neg_x()).cloned();
        let pos_y = structure.chunk_at_unbound(unbound.pos_y()).cloned();
        let neg_y = structure.chunk_at_unbound(unbound.neg_y()).cloned();
        let pos_z = structure.chunk_at_unbound(unbound.pos_z()).cloned();
        let neg_z = structure.chunk_at_unbound(unbound.neg_z()).cloned();

        // "gee, you sure have a way with the borrow checker"

        let materials = materials.clone();
        let blocks = blocks.clone();
        let meshes_registry = meshes_registry.clone();
        let block_textures = block_textures.clone();
        let lighting = lighting.clone();
        let materials_registry = materials_registry.clone();
        let block_rendering_mode = block_rendering_mode.clone();

        let task = async_task_pool.spawn(async move {
            let mut renderer = ChunkRenderer::<CosmosMeshBuilder>::new();

            let chunk_checker = ChunkRenderingChecker {
                neighbors: ChunkNeighbors {
                    neg_x: neg_x.as_ref(),
                    neg_y: neg_y.as_ref(),
                    neg_z: neg_z.as_ref(),
                    pos_x: pos_x.as_ref(),
                    pos_y: pos_y.as_ref(),
                    pos_z: pos_z.as_ref(),
                },
            };

            let custom_blocks = renderer.render(
                &materials.registry(),
                &materials_registry.registry(),
                &lighting.registry(),
                &chunk,
                &blocks.registry(),
                &meshes_registry.registry(),
                &block_rendering_mode,
                &block_textures.registry(),
                &chunk_checker,
                1.0,
                Vec3::ZERO,
                false,
            );

            // let custom_blocks = Default::default();

            ChunkRenderResult {
                chunk_entity: entity,
                custom_blocks,
                // mesh: super::ChunkMesh {
                //     lights: Default::default(),
                //     mesh_materials: Default::default(),
                // },
                mesh: renderer.create_mesh(),
            }
        });

        rendering_chunks.push(RenderingChunk(task));

        commands.entity(entity).remove::<ChunkNeedsRendered>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        ((monitor_needs_rendered_system, poll_rendering_chunks)
            .chain()
            .in_set(StructureRenderingSet::BeginRendering),),
    );
}
