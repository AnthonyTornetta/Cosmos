use std::f32::consts::PI;

use crate::block::blocks::Blocks;
use crate::block::BlockFace;
use crate::events::block_events::BlockChangedEvent;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF};
use crate::structure::events::ChunkSetEvent;
use crate::structure::structure::Structure;
use bevy::prelude::{
    App, Commands, Component, CoreStage, Entity, EventReader, EventWriter, Quat, Query, Res, Vec3,
};
use bevy::utils::HashSet;
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::prelude::{Collider, Rot};

pub struct ChunkPhysicsModel {
    pub collider: Option<Collider>,
    pub chunk_coords: Vector3<usize>,
}

#[derive(Component)]
pub struct StructurePhysics {
    needs_changed: HashSet<Vector3<usize>>,
}

impl StructurePhysics {
    pub fn new(structure: &Structure) -> Self {
        let mut me = Self {
            needs_changed: HashSet::with_capacity(
                structure.chunks_width() * structure.chunks_height() * structure.chunks_length(),
            ),
        };

        for z in 0..structure.chunks_length() {
            for y in 0..structure.chunks_height() {
                for x in 0..structure.chunks_width() {
                    me.needs_changed.insert(Vector3::new(x, y, z));
                }
            }
        }

        me
    }

    pub fn create_colliders(
        &mut self,
        structure: &Structure,
        blocks: &Res<Blocks>,
    ) -> Vec<ChunkPhysicsModel> {
        let mut colliders = Vec::with_capacity(self.needs_changed.len());

        for c in &self.needs_changed {
            if let Some(chunk) = structure.chunk_from_chunk_coordinates(c.x, c.y, c.z) {
                colliders.push(ChunkPhysicsModel {
                    collider: generate_chunk_collider(structure, chunk, blocks),
                    chunk_coords: *c,
                });
            }
        }

        self.needs_changed.clear();

        colliders
    }
}

fn generate_colliders(
    structure: &Structure,
    chunk: &Chunk,
    blocks: &Res<Blocks>,
    colliders: &mut Vec<(Vect, Rot, Collider)>,
    location: Vect,
    offset: Vector3<usize>,
    size: usize,
    density: &mut f32,
    com_divisor: &mut f32,
    com_vec: &mut Vec3,
) {
    let mut last_seen_empty = None;

    let mut temp_com_vec = Vec3::new(0.0, 0.0, 0.0);
    let mut temp_com_divisor = 0.0;
    let mut temp_density = 0.0;

    let half_size = CHUNK_DIMENSIONS as f32 / 2.0;

    for z in offset.z..(offset.z + size) {
        for y in offset.y..(offset.y + size) {
            for x in offset.x..(offset.x + size) {
                let b = blocks.block_from_numeric_id(chunk.block_at(x, y, z));

                temp_density += b.density();

                let mass = b.density(); // 1*1*1*density = density

                temp_com_vec.x += mass * ((x - offset.x) as f32 - half_size);
                temp_com_vec.y += mass * ((y - offset.y) as f32 - half_size);
                temp_com_vec.z += mass * ((z - offset.z) as f32 - half_size);

                temp_com_divisor += mass;

                if last_seen_empty.is_none() {
                    last_seen_empty = Some(b.is_empty());
                } else if last_seen_empty.unwrap() != b.is_empty() {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;
                    // left bottom back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x, offset.y, offset.z),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // right bottom back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // left top back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // left bottom front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x, offset.y, offset.z + s2),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // right bottom front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z + s2),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // left top front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z + s2),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // right top front
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z + s2),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );

                    // right top back
                    generate_colliders(
                        structure,
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z),
                        s2,
                        density,
                        com_divisor,
                        com_vec,
                    );
                    return;
                }
            }
        }
    }

    if !last_seen_empty.unwrap() {
        let s2 = size as f32 / 2.0;

        *density += temp_density;
        *com_divisor += temp_com_divisor;
        *com_vec += temp_com_vec;

        match structure.shape() {
            crate::structure::structure::StructureShape::Flat => {
                colliders.push((location, Rot::IDENTITY, Collider::cuboid(s2, s2, s2)));
            }
            crate::structure::structure::StructureShape::Sphere { radius: _ } => {
                let c2 = CHUNK_DIMENSIONSF / 2.0;

                let (left, bottom, back) = (
                    (location.x + c2 - s2) as usize,
                    (location.y + c2 - s2) as usize,
                    (location.z + c2 - s2) as usize,
                );

                let (right, top, front) = (
                    (location.x + c2 - s2) as usize + size - 1,
                    (location.y + c2 - s2) as usize + size - 1,
                    (location.z + c2 - s2) as usize + size - 1,
                );

                let nnn = get_block_coords(chunk, left, bottom, back, false, false, false);
                let nnp = get_block_coords(chunk, left, bottom, front, false, false, true);
                let npn = get_block_coords(chunk, left, top, back, false, true, false);
                let npp = get_block_coords(chunk, left, top, front, false, true, true);

                let pnn = get_block_coords(chunk, right, bottom, back, true, false, false);
                let pnp = get_block_coords(chunk, right, bottom, front, true, false, true);
                let ppn = get_block_coords(chunk, right, top, back, true, true, false);
                let ppp = get_block_coords(chunk, right, top, front, true, true, true);

                colliders.push((
                    Vec3::ZERO,
                    Rot::IDENTITY,
                    Collider::convex_hull(&[
                        // right
                        pnn, ppn, ppp, pnp,
                        // bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z),
                        // top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z),
                        // top_vec_pos + Vec3::new(cx + top_x, 0.0, cz + top_z),
                        // bot_vec_pos + Vec3::new(cx + bot_x, 0.0, cz + bot_z),
                        // left
                        nnp, npp, npn, nnn,
                        // bot_vec_pos + Vec3::new(cx - bot_x, 0.0, cz + bot_z),
                        // top_vec_pos + Vec3::new(cx - top_x, 0.0, cz + top_z),
                        // top_vec + Vec3::new(cx - top_x, 0.0, cz - top_z),
                        // bot_vec + Vec3::new(cx - bot_x, 0.0, cz - bot_z),
                        // top
                        ppn, npn, npp, ppp,
                        // top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z),
                        // top_vec + Vec3::new(cx - top_x, 0.0, cz - top_z),
                        // top_vec + Vec3::new(cx - top_x, 0.0, cz + top_z),
                        // top_vec + Vec3::new(cx + top_x, 0.0, cz + top_z),
                        // bottom
                        pnp, nnp, nnn, pnn,
                        // bot_vec + Vec3::new(cx + bot_x, 0.0, cz + bot_z),
                        // bot_vec + Vec3::new(cx - bot_x, 0.0, cz + bot_z),
                        // bot_vec + Vec3::new(cx - bot_x, 0.0, cz - bot_z),
                        // bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z),
                        // back
                        nnn, pnn, ppn, npn,
                        // bot_vec + Vec3::new(cx - bot_x, 0.0, cz + bot_z),
                        // bot_vec + Vec3::new(cx + bot_x, 0.0, cz + bot_z),
                        // top_vec + Vec3::new(cx + top_x, 0.0, cz + top_z),
                        // top_vec + Vec3::new(cx - top_x, 0.0, cz + top_z),
                        // front
                        npp, ppp, pnp,
                        nnp,
                        // top_vec_pos + Vec3::new(cx - top_x, 0.0, cz - top_z),
                        // top_vec + Vec3::new(cx + top_x, 0.0, cz - top_z),
                        // bot_vec + Vec3::new(cx + bot_x, 0.0, cz - bot_z),
                        // bot_vec_pos + Vec3::new(cx - bot_x, 0.0, cz - bot_z),
                    ])
                    .expect("Failed to create collider"),
                ));
            }
        }
    }
}

/// Gets the vertex for a block at the given coordinates in a given corner
/// This assumes the chunk is a part of a spherical structure
fn get_block_coords(
    chunk: &Chunk,
    x: usize,
    y: usize,
    z: usize,
    pos_x: bool,
    pos_y: bool,
    pos_z: bool,
) -> Vec3 {
    let y_influence = (y + chunk.structure_y() * CHUNK_DIMENSIONS) as f32;

    let half_curve = (chunk.angle_end_x() - chunk.angle_start_x()) / 2.0;

    let curve_per_block = (chunk.angle_end_z() - chunk.angle_start_z()) / (CHUNK_DIMENSIONSF);

    let theta = PI / 2.0 - curve_per_block;
    let theta_diff = theta.cos();

    // This assumes it is a sphere
    let (bot_x, top_x, bot_y, top_y, bot_z, top_z) = (
        0.5 + (theta_diff * y_influence) / 2.0,
        0.5 + (theta_diff + theta_diff * y_influence) / 2.0,
        0.5,
        0.5,
        0.5 + (theta_diff * y_influence) / 2.0,
        0.5 + (theta_diff + theta_diff * y_influence) / 2.0,
    );

    let quat = Quat::from_euler(
        bevy::prelude::EulerRot::ZYX,
        half_curve - curve_per_block * x as f32,
        0.0,
        -half_curve + curve_per_block * z as f32,
    );

    let (cx, cy, cz) = (
        (x as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5),
        y as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5,
        (z as f32 - CHUNK_DIMENSIONSF / 2.0 + 0.5),
    );

    let cxi = cx; //.floor();
    let cyi = cy; //.floor();
    let czi = cz; //.floor();

    let bot_vec = Vec3::new(0.0, -CHUNK_DIMENSIONSF / 2.0, 0.0)
        + quat.mul_vec3(Vec3::new(0.0, cyi - bot_y + CHUNK_DIMENSIONSF / 2.0, 0.0));
    let top_vec = Vec3::new(0.0, -CHUNK_DIMENSIONSF / 2.0, 0.0)
        + quat.mul_vec3(Vec3::new(0.0, cyi + top_y + CHUNK_DIMENSIONSF / 2.0, 0.0));

    if pos_x && pos_y && pos_z {
        // +x +y +z
        top_vec + Vec3::new(cxi + top_x, 0.0, czi + top_z)
    } else if pos_x && pos_y && !pos_z {
        // +x +y -z
        top_vec + Vec3::new(cxi + top_x, 0.0, czi - top_z)
    } else if pos_x && !pos_y && pos_z {
        // +x -y +z
        bot_vec + Vec3::new(cxi + bot_x, 0.0, czi + bot_z)
    } else if pos_x && !pos_y && !pos_z {
        // +x -y -z
        bot_vec + Vec3::new(cxi + bot_x, 0.0, czi - bot_z)
    } else if !pos_x && pos_y && pos_z {
        // -x +y +z
        top_vec + Vec3::new(cxi - top_x, 0.0, czi + top_z)
    } else if !pos_x && pos_y && !pos_z {
        // -x +y -z
        top_vec + Vec3::new(cxi - top_x, 0.0, czi - top_z)
    } else if !pos_y && !pos_y && pos_z {
        // -x -y +z
        bot_vec + Vec3::new(cxi - bot_x, 0.0, czi + bot_z)
    } else {
        // -x -y -z
        bot_vec + Vec3::new(cxi - bot_x, 0.0, czi - bot_z)
    }
}

fn generate_chunk_collider(
    structure: &Structure,
    chunk: &Chunk,
    blocks: &Res<Blocks>,
) -> Option<Collider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    let mut center_of_mass = Vec3::new(0.0, 0.0, 0.0);
    let mut divisor: f32 = 0.0;
    let mut density: f32 = 0.0;

    generate_colliders(
        structure,
        chunk,
        blocks,
        &mut colliders,
        Vect::new(0.0, 0.0, 0.0),
        Vector3::new(0, 0, 0),
        CHUNK_DIMENSIONS,
        &mut density,
        &mut divisor,
        &mut center_of_mass,
    );

    if divisor != 0.0 {
        center_of_mass.x /= divisor;
        center_of_mass.y /= divisor;
        center_of_mass.z /= divisor;
    }

    if colliders.is_empty() {
        None
    } else {
        Some(Collider::compound(colliders))
    }
}

pub struct NeedsNewPhysicsEvent {
    structure_entity: Entity,
}

fn listen_for_new_physics_event(
    mut commands: Commands,
    mut event: EventReader<NeedsNewPhysicsEvent>,
    mut query: Query<(&Structure, &mut StructurePhysics)>,
    blocks: Res<Blocks>,
) {
    if !event.is_empty() {
        let mut done_structures = HashSet::new();

        for ev in event.iter() {
            if done_structures.contains(&ev.structure_entity.index()) {
                continue;
            }

            done_structures.insert(ev.structure_entity.index());

            let (structure, mut physics) = query.get_mut(ev.structure_entity).unwrap();

            let colliders = physics.create_colliders(structure, &blocks);

            for chunk_collider in colliders {
                let coords = &chunk_collider.chunk_coords;
                if let Some(chunk_entity) = structure.chunk_entity(coords.x, coords.y, coords.z) {
                    let mut entity_commands = commands.entity(chunk_entity);
                    entity_commands.remove::<Collider>();

                    if chunk_collider.collider.is_some() {
                        entity_commands.insert(chunk_collider.collider.unwrap());
                    }
                }
            }
        }
    }
}

fn dew_it(
    done_structures: &mut HashSet<Entity>,
    entity: Entity,
    chunk_coords: Option<Vector3<usize>>,
    query: &mut Query<&mut StructurePhysics>,
    event_writer: &mut EventWriter<NeedsNewPhysicsEvent>,
) {
    if let Some(chunk_coords) = chunk_coords {
        let mut structure_physics = query.get_mut(entity).unwrap();

        structure_physics.needs_changed.insert(chunk_coords);
    }

    if !done_structures.contains(&entity) {
        done_structures.insert(entity);

        event_writer.send(NeedsNewPhysicsEvent {
            structure_entity: entity,
        });
    }
}

fn listen_for_structure_event(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    mut query: Query<&mut StructurePhysics>,
    mut event_writer: EventWriter<NeedsNewPhysicsEvent>,
) {
    let mut done_structures = HashSet::new();
    for ev in event.iter() {
        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(
                ev.block.chunk_coord_x(),
                ev.block.chunk_coord_y(),
                ev.block.chunk_coord_z(),
            )),
            &mut query,
            &mut event_writer,
        );
    }

    for ev in chunk_set_event.iter() {
        dew_it(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(ev.x, ev.y, ev.z)),
            &mut query,
            &mut event_writer,
        );
    }
}

pub fn register(app: &mut App) {
    app.add_event::<NeedsNewPhysicsEvent>()
        .add_system_to_stage(CoreStage::PostUpdate, listen_for_structure_event)
        .add_system_to_stage(CoreStage::PostUpdate, listen_for_new_physics_event);
}
