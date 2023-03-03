use bevy::{
    ecs::system::EntityCommands,
    prelude::{BuildChildren, PbrBundle, Transform},
};
use bevy_rapier3d::prelude::Velocity;

use crate::{
    physics::{location::Location, structure_physics::StructurePhysics},
    structure::Structure,
};

pub trait TStructureBuilder {
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        location: Location,
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
        location: Location,
        velocity: Velocity,
        structure: &mut Structure,
    ) {
        structure.set_entity(entity.id());

        let physics_updater = StructurePhysics::new(structure);

        entity
            .insert(PbrBundle {
                transform: Transform::from_translation(location.local),
                ..Default::default()
            })
            .insert((velocity, location))
            .with_children(|parent| {
                for z in 0..structure.chunks_length() {
                    for y in 0..structure.chunks_height() {
                        for x in 0..structure.chunks_width() {
                            let entity = parent
                                .spawn(PbrBundle {
                                    transform: Transform::from_translation(
                                        structure.chunk_relative_position(x, y, z),
                                    ),
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
