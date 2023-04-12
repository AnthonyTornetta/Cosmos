use bevy::{
    ecs::system::EntityCommands,
    prelude::{PbrBundle, Transform},
};
use bevy_rapier3d::prelude::Velocity;

use crate::{
    physics::{location::Location, structure_physics::StructurePhysics},
    structure::Structure,
};

/// Used to instantiate structures
pub trait TStructureBuilder {
    /// Builds that structure
    fn insert_structure(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        velocity: Velocity,
        structure: &mut Structure,
    );
}
#[derive(Default)]
/// The default structure builder
pub struct StructureBuilder;

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
