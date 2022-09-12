use bevy::{
    ecs::system::EntityCommands,
    prelude::{BuildChildren, PbrBundle, Transform},
};
use bevy_rapier3d::prelude::{RigidBody, Velocity};

use crate::{physics::structure_physics::StructurePhysics, structure::structure::Structure};

use super::planet_builder_trait::TPlanetBuilder;

#[derive(Default)]
pub struct PlanetBuilder {}

impl TPlanetBuilder for PlanetBuilder {
    fn create(&self, entity: &mut EntityCommands, transform: Transform, structure: &mut Structure) {
        let physics_updater = StructurePhysics::new(structure, entity.id());

        entity
            .insert(RigidBody::Fixed)
            .insert_bundle(PbrBundle {
                transform,
                ..Default::default()
            })
            .insert(Velocity::default())
            .with_children(|parent| {
                for z in 0..structure.length() {
                    for y in 0..structure.height() {
                        for x in 0..structure.width() {
                            let entity = parent
                                .spawn()
                                .insert_bundle(PbrBundle {
                                    transform: Transform::from_translation(
                                        structure.chunk_relative_position(x, y, z).into(),
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
