use bevy::{
    ecs::system::EntityCommands,
    prelude::{BuildChildren, PbrBundle, Transform},
};
use bevy_rapier3d::prelude::Velocity;

use crate::{physics::structure_physics::StructurePhysics, structure::structure::Structure};

pub trait TStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        transform: Transform,
        velocity: Velocity,
        structure: &mut Structure,
    );
}
#[derive(Default)]
pub struct StructureBuilder {}

impl TStructureBuilder for StructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        transform: Transform,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        let physics_updater = StructurePhysics::new(structure);

        entity
            .insert_bundle(PbrBundle {
                transform,
                ..Default::default()
            })
            .insert(velocity)
            .with_children(|parent| {
                let render_distance = 2;

                let observer_x_chunk_coords = 0;
                let observer_z_chunk_coords = 0;

                for dz in -render_distance..(render_distance + 1) {
                    for y in 0..structure.chunks_height() {
                        for dx in -render_distance..(render_distance + 1) {
                            let mut xx =
                                (dx + observer_x_chunk_coords) % structure.chunks_width() as i32;
                            let mut zz =
                                (dz + observer_z_chunk_coords) % structure.chunks_length() as i32;

                            if xx < 0 {
                                xx += structure.chunks_width() as i32;
                            }

                            if zz < 0 {
                                zz += structure.chunks_length() as i32;
                            }

                            let z = zz as usize;
                            let x = xx as usize;

                            let (rotation, translation) = structure.chunk_relative_transform(
                                x,
                                y,
                                z,
                                observer_x_chunk_coords,
                                observer_z_chunk_coords,
                            );

                            let entity = parent
                                .spawn()
                                .insert_bundle(PbrBundle {
                                    transform: Transform {
                                        translation,
                                        rotation,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .id();

                            structure.set_chunk_entity(x, y, z, entity);
                        }
                    }
                }
            })
            .insert(physics_updater);
    }
}
