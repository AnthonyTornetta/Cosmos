//! Represents all the systems a structure has. You should access systems a specific structure has
//! through this. It is, however, safe to query systems normally if you don't need a specific structure.
//! If you need information about the structure a system belongs to and you are querying through systems, include
//! the `StructureSystem` component to your query to get the structure's entity.
//!
//! Each system is stored as a child of this.

use std::{error::Error, fmt::Formatter};

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};

use super::{loading::StructureLoadingSet, ship::Ship, Structure};

pub mod energy_generation_system;
pub mod energy_storage_system;
pub mod laser_cannon_system;
pub mod line_system;
pub mod mining_laser_system;
pub mod thruster_system;

#[derive(Component)]
#[component(storage = "SparseSet")]
/// Used to tell if the selected system should be active
/// (ie laser cannons firing)
///
/// This component will be on the system's entity
///
/// For example:
///
/// ```rs
/// Query<&LaserCannonSystem, With<SystemActive>>
/// ```
///
/// would give you every laser cannon system that is currently being activated.
pub struct SystemActive;

#[derive(Component)]
/// Used to tell if a system has a specified controller
/// This does not need to be provided if no controller is used
pub struct SystemBlock(String);

#[derive(Component, Debug, Reflect)]
/// Every system has this as a component.
pub struct StructureSystem {
    structure_entity: Entity,
    system_id: StructureSystemId,
}

impl StructureSystem {
    /// This system's unique id
    pub fn id(&self) -> StructureSystemId {
        self.system_id
    }

    /// The entity this system belongs to
    pub fn structure_entity(&self) -> Entity {
        self.structure_entity
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Reflect, Hash)]
/// Uniquely identifies a system on a per-structure basis.
///
/// This can have collisions across multiple structures, but is guarenteed to be unique per-structure.
pub struct StructureSystemId(u64);

impl StructureSystemId {
    /// Creates a new system id.
    ///
    /// This does not check for collisions
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug)]
/// If no system was found, this error will be returned.
pub struct NoSystemFound;

impl std::fmt::Display for NoSystemFound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "No suitable system was found")
    }
}

impl Error for NoSystemFound {}

#[derive(Component)]
/// Stores all the systems a structure has
pub struct Systems {
    /// These entities should have the `StructureSystem` component
    pub systems: Vec<Entity>,
    /// The system ids
    ids: HashMap<StructureSystemId, usize>,
    /// More than just one system can be active at a time, but the pilot can only personally activate one system at a time
    /// Perhaps make this a component on the pilot entity in the future?
    /// Currently this limits a ship to one pilot, the above would fix this issue, but this is a future concern.
    active_system: Option<u32>,
    entity: Entity,
}

impl Systems {
    fn has_id(&self, id: StructureSystemId) -> bool {
        self.ids.contains_key(&id)
    }

    fn insert_system(&mut self, system_id: StructureSystemId, entity: Entity) {
        let idx = self.systems.len();
        self.ids.insert(system_id, idx);
        self.systems.push(entity);
    }

    /// Gets the entity that corresponds to the system id, or none if not found.
    pub fn get_system_entity(&self, system_id: StructureSystemId) -> Option<Entity> {
        self.ids
            .get(&system_id)
            .copied()
            .map(|idx| self.systems.get(idx))
            .flatten()
            .copied()
    }

    /// Activates the passed in selected system, and deactivates the system that was previously selected
    pub fn set_active_system(&mut self, active: Option<u32>, commands: &mut Commands) {
        if active == self.active_system {
            return;
        }

        if let Some(active_system) = self.active_system {
            if (active_system as usize) < self.systems.len() {
                commands.entity(self.systems[active_system as usize]).remove::<SystemActive>();
            }
        }

        if let Some(active_system) = active {
            if (active_system as usize) < self.systems.len() {
                commands.entity(self.systems[active_system as usize]).insert(SystemActive);

                self.active_system = active;
            } else {
                self.active_system = None;
            }
        } else {
            self.active_system = None;
        }
    }

    /// Returns the active system entity, if there is one.
    pub fn active_system(&self) -> Option<Entity> {
        self.active_system.map(|x| self.systems[x as usize])
    }

    /// Generates a new id for a system while avoiding collisions
    fn generate_new_system_id(&self) -> StructureSystemId {
        let mut system_id;

        loop {
            system_id = StructureSystemId::new(rand::random::<u64>());
            if !self.has_id(system_id) {
                break;
            }
        }

        system_id
    }

    /// Adds a system to the structure. Use this instead of directly adding it with commands.
    pub fn add_system<T: Component>(&mut self, commands: &mut Commands, system: T) -> Entity {
        let mut ent = None;

        let system_id = self.generate_new_system_id();

        commands.entity(self.entity).with_children(|p| {
            let entity = p
                .spawn(system)
                .insert(StructureSystem {
                    structure_entity: self.entity,
                    system_id,
                })
                .id();

            self.insert_system(system_id, entity);

            ent = Some(entity);
        });

        ent.expect("This should have been set in above closure.")
    }

    /// Queries all the systems of a structure with this specific query, or returns `Err(NoSystemFound)` if none matched this query.
    ///
    /// TODO: in future allow for this to take any number of components
    pub fn query<'a, T: Component>(&'a self, query: &'a Query<&T>) -> Result<&T, NoSystemFound> {
        for ent in self.systems.iter() {
            if let Ok(res) = query.get(*ent) {
                return Ok(res);
            }
        }

        Err(NoSystemFound)
    }

    /// Queries all the systems of a structure with this specific query, or returns `Err(NoSystemFound)` if none matched this query.
    ///
    /// TODO: in future allow for this to take any number of components
    pub fn query_mut<'a, T: Component>(&'a self, query: &'a mut Query<&mut T>) -> Result<Mut<T>, NoSystemFound> {
        for ent in self.systems.iter() {
            // for some reason, the borrow checker gets mad when I do a get_mut in this if statement
            if query.get(*ent).is_ok() {
                return Ok(query.get_mut(*ent).expect("This should be valid"));
            }
        }

        Err(NoSystemFound)
    }
}

fn add_structure(mut commands: Commands, query: Query<Entity, (Added<Structure>, With<Ship>)>) {
    for entity in query.iter() {
        commands.entity(entity).insert(Systems {
            systems: Vec::new(),
            entity,
            active_system: None,
            ids: Default::default(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, add_structure.in_set(StructureLoadingSet::LoadChunkData))
        .register_type::<StructureSystem>();

    line_system::register(app);
    energy_storage_system::register(app);
    energy_generation_system::register(app);
    thruster_system::register(app);
}
