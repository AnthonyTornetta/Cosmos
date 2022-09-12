use bevy::ecs::system::EntityCommands;
use bevy::prelude::Transform;
use bevy_rapier3d::prelude::Velocity;

use crate::structure::structure::Structure;

pub trait TStructureBuilder {
    fn create(
        &self,
        entity: &mut EntityCommands,
        transform: Transform,
        velocity: Velocity,
        structure: &mut Structure,
    );
}
