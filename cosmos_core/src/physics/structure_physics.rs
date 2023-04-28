//! Responsible for the collider generation of a structure.

use crate::block::Block;
use crate::events::block_events::BlockChangedEvent;
use crate::registry::Registry;
use crate::structure::chunk::{Chunk, CHUNK_DIMENSIONS};
use crate::structure::events::ChunkSetEvent;
use crate::structure::Structure;
use bevy::prelude::{
    Added, App, Commands, Component, Entity, EventReader, EventWriter, Query, Res, Vec3, Without,
};
use bevy::reflect::{FromReflect, Reflect};
use bevy::utils::HashSet;
use bevy_rapier3d::math::Vect;
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::prelude::{Collider, ColliderMassProperties, ReadMassProperties, Rot};

type GenerateCollider = (Collider, f32, Vec3);

struct ChunkPhysicsModel {
    collider: Option<GenerateCollider>,
    chunk_coords: Vector3<usize>,
}

#[derive(Component, Debug, Reflect, FromReflect)]
pub struct StructurePhysics {
    needs_changed: HashSet<Vector3<usize>>,
}

fn add_physics(
    query: Query<(Entity, &Structure), (Added<Structure>, Without<StructurePhysics>)>,
    mut commands: Commands,
) {
    for (entity, structure) in query.iter() {
        let physics_updater = StructurePhysics::new(structure);

        commands.entity(entity).insert(physics_updater);
    }
}

/// Sometimes the ReadMassProperties is wrong, so this component fixes it
#[derive(Component, Debug, Reflect, FromReflect, PartialEq, Clone, Copy)]
struct StructureMass {
    mass: f32,
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

    fn create_colliders(
        &mut self,
        structure: &Structure,
        blocks: &Registry<Block>,
    ) -> Vec<ChunkPhysicsModel> {
        let mut colliders = Vec::with_capacity(self.needs_changed.len());

        for c in &self.needs_changed {
            if let Some(chunk) = structure.chunk_from_chunk_coordinates(c.x, c.y, c.z) {
                colliders.push(ChunkPhysicsModel {
                    collider: generate_chunk_collider(chunk, blocks),
                    chunk_coords: *c,
                });
            }
        }

        self.needs_changed.clear();

        colliders
    }
}

/// This works by first checking if the cube that is within its bounds contains either all solid or empty blocks
///
/// If it does, this exits either creating a single cube collider for that or no collider
///
/// Otherwise, this recursively calls itself 8 times breaking this large cube into 8 equally sized smaller
/// cubes and repeating this process.
///
/// This prevents the creation of tons of small colliders while being relatively easy to implement.
fn generate_colliders(
    chunk: &Chunk,
    blocks: &Registry<Block>,
    colliders: &mut Vec<(Vect, Rot, Collider)>,
    location: Vect,
    offset: Vector3<usize>,
    size: usize,
    mass: &mut f32,
    com_divisor: &mut f32,
    com_vec: &mut Vec3,
) {
    let mut last_seen_empty = None;

    let mut temp_com_vec = Vec3::new(0.0, 0.0, 0.0);
    let mut temp_com_divisor = 0.0;
    let mut temp_mass = 0.0;

    let half_size = CHUNK_DIMENSIONS as f32 / 2.0;

    for z in offset.z..(offset.z + size) {
        for y in offset.y..(offset.y + size) {
            for x in offset.x..(offset.x + size) {
                let b = blocks.from_numeric_id(chunk.block_at(x, y, z));

                let block_mass = b.density(); // mass = volume * density = 1*1*1*density = density

                if block_mass != 0.0 {
                    temp_mass += block_mass;

                    let (xx, yy, zz) = (
                        (x - offset.x) as f32 - half_size + 0.5,
                        (y - offset.y) as f32 - half_size + 0.5,
                        (z - offset.z) as f32 - half_size + 0.5,
                    );

                    temp_com_vec.x += block_mass * xx;
                    temp_com_vec.y += block_mass * yy;
                    temp_com_vec.z += block_mass * zz;

                    temp_com_divisor += block_mass;
                }

                if last_seen_empty.is_none() {
                    last_seen_empty = Some(b.is_empty());
                } else if last_seen_empty.unwrap() != b.is_empty() {
                    let s2 = size / 2;
                    let s4 = s2 as f32 / 2.0;

                    // left bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x, offset.y, offset.z),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // right bottom back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // left top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // left bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x, offset.y, offset.z + s2),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // right bottom front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y - s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y, offset.z + s2),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // left top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x - s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // right top front
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z + s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z + s2),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );

                    // right top back
                    generate_colliders(
                        chunk,
                        blocks,
                        colliders,
                        Vect::new(location.x + s4, location.y + s4, location.z - s4),
                        Vector3::new(offset.x + s2, offset.y + s2, offset.z),
                        s2,
                        mass,
                        com_divisor,
                        com_vec,
                    );
                    return;
                }
            }
        }
    }

    // If this is true, then this cube was fully empty.
    if !last_seen_empty.unwrap() {
        let s2 = size as f32 / 2.0;

        *mass += temp_mass;
        *com_divisor += temp_com_divisor;
        *com_vec += temp_com_vec;

        colliders.push((location, Rot::IDENTITY, Collider::cuboid(s2, s2, s2)));
    }
}

fn generate_chunk_collider(chunk: &Chunk, blocks: &Registry<Block>) -> Option<GenerateCollider> {
    let mut colliders: Vec<(Vect, Rot, Collider)> = Vec::new();

    let mut center_of_mass = Vec3::new(0.0, 0.0, 0.0);
    let mut divisor: f32 = 0.0;
    let mut mass: f32 = 0.0;

    generate_colliders(
        chunk,
        blocks,
        &mut colliders,
        Vect::new(0.0, 0.0, 0.0),
        Vector3::new(0, 0, 0),
        CHUNK_DIMENSIONS,
        &mut mass,
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
        Some((Collider::compound(colliders), mass, center_of_mass))
    }
}

/// Sent when a structure needs new physics
pub struct NeedsNewPhysicsEvent {
    structure_entity: Entity,
}

/// This system is responsible for adding colliders to chunks
pub fn listen_for_new_physics_event(
    mut commands: Commands,
    mut event: EventReader<NeedsNewPhysicsEvent>,
    mut query: Query<(&Structure, &mut StructurePhysics)>,
    blocks: Res<Registry<Block>>,
) {
    if !event.is_empty() {
        let mut done_structures = HashSet::new();

        for ev in event.iter() {
            if done_structures.contains(&ev.structure_entity) {
                continue;
            }

            done_structures.insert(ev.structure_entity);

            let (structure, mut physics) = query.get_mut(ev.structure_entity).unwrap();

            let colliders = physics.create_colliders(structure, &blocks);

            for chunk_collider in colliders {
                let coords = &chunk_collider.chunk_coords;

                if let Some(chunk_entity) = structure.chunk_entity(coords.x, coords.y, coords.z) {
                    let mut entity_commands = commands.entity(chunk_entity);
                    if let Some((collider, mass, _)) = chunk_collider.collider {
                        // center_of_mass needs custom torque calculations to work properly

                        // let mass_props = MassProperties {
                        //     mass,
                        //     // local_center_of_mass,
                        //     ..Default::default()
                        // };

                        entity_commands
                            .insert(collider)
                            .insert(ColliderMassProperties::Mass(mass));
                        // .insert(ColliderMassProperties::MassProperties(mass_props))
                        // Sometimes this gets out-of-sync, so I update it manually here
                        // .insert(ReadMassProperties(mass_props));
                    } else {
                        entity_commands.remove::<Collider>();
                    }
                }
            }
        }
    }
}

/// Marks a structure for change if it already hasn't been marked
fn maybe_mark_structure_for_change(
    done_structures: &mut HashSet<Entity>,
    entity: Entity,
    chunk_coords: Option<Vector3<usize>>,
    query: &mut Query<&mut StructurePhysics>,
    event_writer: &mut EventWriter<NeedsNewPhysicsEvent>,
) {
    if let Some(chunk_coords) = chunk_coords {
        let Ok(mut structure_physics) = query.get_mut(entity) else {
            // Entity may have been unloaded
            return;
        };

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
        maybe_mark_structure_for_change(
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
        maybe_mark_structure_for_change(
            &mut done_structures,
            ev.structure_entity,
            Some(Vector3::new(ev.x, ev.y, ev.z)),
            &mut query,
            &mut event_writer,
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<NeedsNewPhysicsEvent>()
        // This wasn't registered in bevy_rapier
        .register_type::<ReadMassProperties>()
        .register_type::<ColliderMassProperties>()
        .add_systems((
            listen_for_structure_event,
            listen_for_new_physics_event,
            add_physics,
        ));
}

#[cfg(test)]
mod test {
    use bevy::prelude::Vec3;

    use crate::{
        block::{block_builder::BlockBuilder, Block},
        registry::Registry,
        structure::chunk::{Chunk, CHUNK_DIMENSIONS, CHUNK_DIMENSIONSF},
    };

    use super::generate_chunk_collider;

    #[test]
    fn test_gen_colliders_one_block() {
        let mut chunk = Chunk::new(10, 10, 10);
        let mut blocks = Registry::<Block>::new();

        blocks.register(BlockBuilder::new("air".into(), 0.0).create());
        blocks.register(BlockBuilder::new("test".into(), 4.0).create());

        let test_block = blocks.from_id("test").unwrap();

        chunk.set_block_at(1, 2, 3, test_block);

        let (_, mass, center_of_mass) = generate_chunk_collider(&chunk, &blocks).unwrap();

        assert_eq!(mass, 4.0);
        assert_eq!(center_of_mass, Vec3::new(-6.5, -5.5, -4.5));
    }

    #[test]
    fn test_gen_colliders_two_same_blocks() {
        let mut chunk = Chunk::new(10, 10, 10);
        let mut blocks = Registry::<Block>::new();

        blocks.register(BlockBuilder::new("air".into(), 0.0).create());
        blocks.register(BlockBuilder::new("test".into(), 4.0).create());

        let test_block = blocks.from_id("test").unwrap();

        chunk.set_block_at(1, 2, 3, test_block);

        chunk.set_block_at(
            CHUNK_DIMENSIONS - 2,
            CHUNK_DIMENSIONS - 3,
            CHUNK_DIMENSIONS - 4,
            test_block,
        );

        let (_, mass, center_of_mass) = generate_chunk_collider(&chunk, &blocks).unwrap();

        assert_eq!(mass, 8.0);
        assert_eq!(center_of_mass, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_gen_colliders_two_diff_blocks() {
        let mut chunk = Chunk::new(10, 10, 10);
        let mut blocks = Registry::<Block>::new();

        blocks.register(BlockBuilder::new("air".into(), 0.0).create());
        blocks.register(BlockBuilder::new("test".into(), 4.0).create());
        blocks.register(BlockBuilder::new("test2".into(), 1.0).create());

        let test_block = blocks.from_id("test").unwrap();
        let test_block_2 = blocks.from_id("test2").unwrap();

        chunk.set_block_at(0, 0, 0, test_block);

        chunk.set_block_at(
            CHUNK_DIMENSIONS - 1,
            CHUNK_DIMENSIONS - 1,
            CHUNK_DIMENSIONS - 1,
            test_block_2,
        );

        let (_, mass, center_of_mass) = generate_chunk_collider(&chunk, &blocks).unwrap();

        assert_eq!(mass, 5.0);
        assert_eq!(
            center_of_mass,
            Vec3::new(
                -CHUNK_DIMENSIONSF / 4.0 - 0.5,
                -CHUNK_DIMENSIONSF / 4.0 - 0.5,
                -CHUNK_DIMENSIONSF / 4.0 - 0.5
            )
        );
    }
}
