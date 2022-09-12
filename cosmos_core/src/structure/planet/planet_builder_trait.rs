use bevy::ecs::system::EntityCommands;
use bevy::prelude::Transform;

use crate::structure::structure::Structure;

pub trait TPlanetBuilder {
    fn create(&self, entity: &mut EntityCommands, transform: Transform, structure: &mut Structure);
}
