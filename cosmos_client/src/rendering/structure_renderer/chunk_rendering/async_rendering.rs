use crate::asset::asset_loading::BlockTextureIndex;
use crate::asset::materials::{AddMaterialEvent, BlockMaterialMapping, MaterialDefinition, MaterialType, RemoveAllMaterialsEvent};
use crate::block::lighting::BlockLighting;
use crate::rendering::structure_renderer::{BlockRenderingModes, StructureRenderingSet};
use crate::rendering::ReadOnlyBlockMeshRegistry;
use bevy::prelude::{
    App, Assets, BuildChildren, Commands, DespawnRecursiveExt, Entity, EventWriter, GlobalTransform, Handle, IntoSystemConfigs, Mesh,
    PointLight, PointLightBundle, Query, Res, ResMut, Transform, Update, Vec3, VisibilityBundle, With,
};
use bevy::render::primitives::Aabb;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::transform::TransformBundle;
use cosmos_core::block::Block;
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::physics::location::SECTOR_DIMENSIONS;
use cosmos_core::registry::many_to_one::ReadOnlyManyToOneRegistry;
use cosmos_core::registry::ReadOnlyRegistry;
use cosmos_core::structure::chunk::{ChunkEntity, CHUNK_DIMENSIONS};
use cosmos_core::structure::coordinates::UnboundChunkCoordinate;
use cosmos_core::structure::Structure;
use futures_lite::future;

use super::chunk_renderer::{ChunkNeedsCustomBlocksRendered, ChunkRenderer, RenderingChunk, RenderingChunks};
use super::{ChunkMeshes, ChunkNeedsRendered, ChunkRenderResult, LightEntry, LightsHolder};

fn poll_rendering_chunks(
    mut rendering_chunks: ResMut<RenderingChunks>,
    mut commands: Commands,
    mesh_query: Query<Option<&Handle<Mesh>>>,
    mut meshes: ResMut<Assets<Mesh>>,
    lights_query: Query<&LightsHolder>,
    chunk_meshes_query: Query<&ChunkMeshes>,
    mut event_writer: EventWriter<AddMaterialEvent>,
    mut remove_all_materials: EventWriter<RemoveAllMaterialsEvent>,
    mut ev_writer: EventWriter<ChunkNeedsCustomBlocksRendered>,
    q_chunk_entity: Query<&ChunkEntity>,
) {
    let mut todo = Vec::with_capacity(rendering_chunks.capacity());

    std::mem::swap(&mut rendering_chunks.0, &mut todo);

    for mut rendering_chunk in todo {
        if let Some(rendered_chunk) = future::block_on(future::poll_once(&mut rendering_chunk.0)) {
            let (entity, mut chunk_mesh) = (rendered_chunk.chunk_entity, rendered_chunk.mesh);

            if commands.get_entity(entity).is_none() {
                // Chunk may have been despawned during its rendering
                continue;
            }

            if let Ok(chunk_entity) = q_chunk_entity.get(entity) {
                // This should be sent even if there are no custom blocks, because the chunk may have had
                // custom blocks in the past that need their rendering info cleaned up
                ev_writer.send(ChunkNeedsCustomBlocksRendered {
                    block_ids: rendered_chunk.custom_blocks,
                    chunk_coordinate: chunk_entity.chunk_location,
                    mesh_entity_parent: entity,
                    structure_entity: chunk_entity.structure_entity,
                });
            }

            let mut old_mesh_entities = Vec::new();

            if let Ok(chunk_meshes_component) = chunk_meshes_query.get(entity) {
                for ent in chunk_meshes_component.0.iter() {
                    let old_mesh_handle = mesh_query.get(*ent).expect("This should have a mesh component.");

                    if let Some(old_mesh_handle) = old_mesh_handle {
                        meshes.remove(old_mesh_handle);
                    }

                    old_mesh_entities.push(*ent);
                }
            }

            let mut new_lights = LightsHolder::default();

            if let Ok(lights) = lights_query.get(entity) {
                for light in lights.lights.iter() {
                    let mut light = *light;
                    light.valid = false;
                    new_lights.lights.push(light);
                }
            }

            let mut entities_to_add = Vec::new();

            if !chunk_mesh.lights.is_empty() {
                for light in chunk_mesh.lights {
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
                            .spawn(PointLightBundle {
                                point_light: PointLight {
                                    color: properties.color,
                                    intensity: properties.intensity,
                                    range: properties.range,
                                    radius: 1.0,
                                    // Shadows kill all performance
                                    shadows_enabled: false, // !properties.shadows_disabled,
                                    ..Default::default()
                                },
                                transform: Transform::from_xyz(
                                    block_light_coord.x as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    block_light_coord.y as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                    block_light_coord.z as f32 - (CHUNK_DIMENSIONS as f32 / 2.0 - 0.5),
                                ),
                                ..Default::default()
                            })
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
                commands.entity(light.entity).despawn_recursive();
            }

            new_lights.lights.retain(|x| x.valid);

            // end lighting
            // meshes

            // If the chunk previously only had one chunk mesh, then it would be on
            // the chunk entity instead of child entities
            commands.entity(entity).remove::<Handle<Mesh>>();

            remove_all_materials.send(RemoveAllMaterialsEvent { entity });

            let mut chunk_meshes_component = ChunkMeshes::default();

            if chunk_mesh.mesh_materials.len() > 1 {
                for mesh_material in chunk_mesh.mesh_materials {
                    let mesh = meshes.add(mesh_material.mesh);

                    let s = (CHUNK_DIMENSIONS / 2) as f32;

                    let ent = commands
                        .spawn((
                            mesh,
                            TransformBundle::default(),
                            VisibilityBundle::default(),
                            // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                            Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                        ))
                        .id();

                    entities_to_add.push(ent);

                    event_writer.send(AddMaterialEvent {
                        entity: ent,
                        add_material_id: mesh_material.material_id,
                        material_type: MaterialType::Normal,
                    });

                    chunk_meshes_component.0.push(ent);
                }
            } else if !chunk_mesh.mesh_materials.is_empty() {
                // To avoid making too many entities (and tanking performance), if only one mesh
                // is present, just stick the mesh info onto the chunk itself.

                let mesh_material = chunk_mesh.mesh_materials.pop().expect("This has one element in it");

                let mesh = meshes.add(mesh_material.mesh);
                let s = (CHUNK_DIMENSIONS / 2) as f32;

                commands.entity(entity).insert((
                    mesh,
                    // mesh_material.material_id,
                    // Remove this once https://github.com/bevyengine/bevy/issues/4294 is done (when bevy ~0.10~ ~0.11~ ~0.12~ 0.13 is released)
                    Aabb::from_min_max(Vec3::new(-s, -s, -s), Vec3::new(s, s, s)),
                ));

                event_writer.send(AddMaterialEvent {
                    entity,
                    add_material_id: mesh_material.material_id,
                    material_type: MaterialType::Normal,
                });
            }

            // Any leftover entities are useless now, so kill them
            for mesh in old_mesh_entities {
                commands.entity(mesh).despawn_recursive();
            }

            let mut entity_commands = commands.entity(entity);

            for ent in entities_to_add {
                entity_commands.add_child(ent);
            }

            entity_commands
                // .insert(meshes.add(chunk_mesh.mesh))
                .insert(new_lights)
                .insert(chunk_meshes_component);
        } else {
            rendering_chunks.push(rendering_chunk);
        }
    }
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
    let Ok(local_transform) = local_player.get_single() else {
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

        let left = structure.chunk_at_unbound(unbound.left()).cloned();
        let right = structure.chunk_at_unbound(unbound.right()).cloned();
        let bottom = structure.chunk_at_unbound(unbound.bottom()).cloned();
        let top = structure.chunk_at_unbound(unbound.top()).cloned();
        let back = structure.chunk_at_unbound(unbound.back()).cloned();
        let front = structure.chunk_at_unbound(unbound.front()).cloned();

        // "gee, you sure have a way with the borrow checker"

        let materials = materials.clone();
        let blocks = blocks.clone();
        let meshes_registry = meshes_registry.clone();
        let block_textures = block_textures.clone();
        let lighting = lighting.clone();
        let materials_registry = materials_registry.clone();
        let block_rendering_mode = block_rendering_mode.clone();

        let task = async_task_pool.spawn(async move {
            let mut renderer = ChunkRenderer::new();

            let custom_blocks = renderer.render(
                &materials.registry(),
                &materials_registry.registry(),
                &lighting.registry(),
                &chunk,
                left.as_ref(),
                right.as_ref(),
                bottom.as_ref(),
                top.as_ref(),
                back.as_ref(),
                front.as_ref(),
                &blocks.registry(),
                &meshes_registry.registry(),
                &block_rendering_mode,
                &block_textures.registry(),
            );

            ChunkRenderResult {
                chunk_entity: entity,
                custom_blocks,
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
