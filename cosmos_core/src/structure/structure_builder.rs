use bevy::{
    ecs::system::EntityCommands,
    prelude::{PbrBundle, Transform},
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
            .insert(physics_updater);
    }
}
