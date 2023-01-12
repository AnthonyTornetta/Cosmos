use bevy::{ecs::schedule::StateData, prelude::*};

use super::Structure;

pub mod energy_generation_system;
pub mod energy_storage_system;
pub mod laser_cannon_system;
pub mod thruster_system;

#[derive(Component)]
#[component(storage = "SparseSet")]
/// Used to tell if the selected system should be active
/// (ie laser cannons firing)
pub struct SystemActive;

#[derive(Component)]
/// Used to tell if a system has a specified controller
/// This does not need to be provided if no controller is used
pub struct SystemBlock(String);

#[derive(Component)]
pub struct System {
    pub structure_entity: Entity,
}

#[derive(Component)]
pub struct Systems {
    /// These entities should have the "System" component
    pub systems: Vec<Option<Entity>>,
    /// More than just one system can be active at a time, but the pilot can only personally activate one system at a time
    /// Perhaps make this a component on the pilot entity in the future?
    /// Currently this limits a ship to one pilot, the above would fix this issue, but idk if it's worth it.
    active_system: Option<usize>,
    entity: Entity,
}

impl Systems {
    pub fn add_system<T: Component>(&mut self, commands: &mut Commands, system: T) -> Entity {
        let ent;

        commands.entity(self.entity).with_children(|p| {
            ent = p
                .spawn(system)
                .insert(System {
                    structure_entity: self.entity,
                })
                .id();
        });

        ent
    }
}

pub fn add_structure(mut commands: Commands, query: Query<Entity, Added<Structure>>) {
    for entity in query.iter() {
        let mut systems = Vec::with_capacity(9 * 10);

        for _ in 0..systems.capacity() {
            systems.push(None);
        }

        commands.entity(entity).insert(Systems {
            systems,
            entity,
            active_system: None,
        });
    }
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    energy_storage_system::register(app, post_loading_state, playing_state);
    energy_generation_system::register(app, post_loading_state, playing_state);
    thruster_system::register(app, post_loading_state, playing_state);
    laser_cannon_system::register(app, post_loading_state, playing_state);
}
