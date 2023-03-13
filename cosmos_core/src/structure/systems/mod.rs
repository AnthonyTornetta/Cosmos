use std::{error::Error, fmt::Formatter};

use bevy::prelude::*;

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
pub struct StructureSystem {
    pub structure_entity: Entity,
}

#[derive(Debug)]
pub struct NoSystemFound;

impl std::fmt::Display for NoSystemFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "No suitable system was found")
    }
}

impl Error for NoSystemFound {}

#[derive(Component)]
pub struct Systems {
    /// These entities should have the "System" component
    pub systems: Vec<Entity>,
    /// More than just one system can be active at a time, but the pilot can only personally activate one system at a time
    /// Perhaps make this a component on the pilot entity in the future?
    /// Currently this limits a ship to one pilot, the above would fix this issue, but idk if it's worth it.
    active_system: Option<u32>,
    entity: Entity,
}

impl Systems {
    pub fn set_active_system(&mut self, active: Option<u32>, commands: &mut Commands) {
        if active == self.active_system {
            return;
        }

        if let Some(active_system) = self.active_system {
            if (active_system as usize) < self.systems.len() {
                commands
                    .entity(self.systems[active_system as usize])
                    .remove::<SystemActive>();
            }
        }

        if let Some(active_system) = active {
            if (active_system as usize) < self.systems.len() {
                commands
                    .entity(self.systems[active_system as usize])
                    .insert(SystemActive);
            }
        }

        self.active_system = active;
    }

    pub fn add_system<T: Component>(&mut self, commands: &mut Commands, system: T) -> Entity {
        let mut ent = None;

        commands.entity(self.entity).with_children(|p| {
            ent = Some(
                p.spawn(system)
                    .insert(StructureSystem {
                        structure_entity: self.entity,
                    })
                    .id(),
            );

            self.systems.push(ent.unwrap());
        });

        ent.expect("This should have been set in above closure.")
    }

    /// TODO: in future allow for this to take any number of components
    pub fn query<'a, T: Component>(&'a self, query: &'a Query<&T>) -> Result<&T, NoSystemFound> {
        for ent in self.systems.iter() {
            if let Ok(res) = query.get(*ent) {
                return Ok(res);
            }
        }

        Err(NoSystemFound)
    }

    /// TODO: in future allow for this to take any number of components
    pub fn query_mut<'a, T: Component>(
        &'a self,
        query: &'a mut Query<&mut T>,
    ) -> Result<Mut<T>, NoSystemFound> {
        for ent in self.systems.iter() {
            // for some reason, the borrow checker gets mad when I do a get_mut in this if statement
            if query.get(*ent).is_ok() {
                return Ok(query.get_mut(*ent).expect("This should be valid"));
            }
        }

        Err(NoSystemFound)
    }
}

fn add_structure(mut commands: Commands, query: Query<Entity, Added<Structure>>) {
    for entity in query.iter() {
        commands.entity(entity).insert(Systems {
            systems: Vec::new(),
            entity,
            active_system: None,
        });
    }
}

pub fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.add_system(add_structure);

    energy_storage_system::register(app, post_loading_state, playing_state);
    energy_generation_system::register(app, post_loading_state, playing_state);
    thruster_system::register(app, post_loading_state, playing_state);
    laser_cannon_system::register(app, post_loading_state, playing_state);
}
