//! Responsible for determining how structures are added to the game when they are needed

use bevy::{ecs::system::EntityCommands, prelude::PbrBundle};
use bevy_rapier3d::prelude::Velocity;

use crate::{physics::location::Location, structure::Structure};

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
#[derive(Default, Debug)]
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

        entity.insert((
            velocity,
            location,
            PbrBundle {
                ..Default::default()
            },
        ));
    }
}
