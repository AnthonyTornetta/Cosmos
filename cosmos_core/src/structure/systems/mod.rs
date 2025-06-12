//! Represents all the systems a structure has. You should access systems a specific structure has
//! through this.
//!
//! It is, however, safe to query systems normally if you don't need a specific structure.
//! If you need information about the structure a system belongs to and you are querying through systems, include
//! the `StructureSystem` component to your query to get the structure's entity.
//!
//! Each system is stored as a child of this.

use std::{error::Error, fmt::Formatter};

use bevy::{
    ecs::query::{QueryData, QueryFilter, QueryItem, ROQueryItem},
    prelude::*,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::registry::sync_registry,
    registry::{Registry, create_registry, identifiable::Identifiable},
};

use super::{Structure, loading::StructureLoadingSet, shared::MeltingDown, ship::Ship};

pub mod camera_system;
pub mod dock_system;
pub mod energy_generation_system;
pub mod energy_storage_system;
pub mod laser_cannon_system;
pub mod line_system;
pub mod mining_laser_system;
pub mod missile_launcher_system;
pub mod railgun_system;
pub mod shield_system;
pub mod sync;
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

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Reflect)]
/// Sets the system the player has selected
pub enum ShipActiveSystem {
    /// No system hovered/active
    #[default]
    None,
    /// A system is being hovered by the user, but is not being activated.
    ///
    /// (Usefor for missile that need time to focus before being used)
    Hovered(u32),
    /// The user is actively firing the system
    Active(u32),
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// Holds some of the logic of Structure systems
pub enum StructureSystemsSet {
    /// Initialize your structure systems here (from [`super::events::StructureLoadedEvent`]s being generated)
    InitSystems,
    /// Update systems when new blocks are placed (from [`crate::events::block_events::BlockChangedEvent`]s)
    UpdateSystemsBlocks,
    /// Update systems post block placement.
    UpdateSystems,
}

fn remove_system_actives_when_melting_down(
    mut commands: Commands,
    q_system_active: Query<Entity, With<SystemActive>>,
    q_melting_down: Query<&StructureSystems, With<MeltingDown>>,
) {
    for systems in &q_melting_down {
        let Ok(ent) = systems.query(&q_system_active) else {
            continue;
        };

        commands.entity(ent).remove::<SystemActive>();
    }
}

#[derive(Component)]
/// Used to tell if a system has a specified controller
/// This does not need to be provided if no controller is used
pub struct SystemBlock;

#[derive(Component, Debug, Reflect)]
/// Every system has this as a component.
pub struct StructureSystem {
    structure_entity: Entity,
    system_id: StructureSystemId,
    system_type_id: StructureSystemTypeId,
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

    /// Gets the type id of the system. This links it to the datatype of this system used for serialization/deserialization
    pub fn system_type_id(&self) -> StructureSystemTypeId {
        self.system_type_id
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Reflect, Hash)]
/// Uniquely identifies a system on a per-structure basis.
///
/// This can have collisions across multiple structures, but is guarenteed to be unique per-structure.
pub struct StructureSystemId(u64);

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Reflect, Hash, Default)]
/// The id of this structure system type
///
/// This will be unique across all systems, and is primarily used for serializing/deserializing structure systems
pub struct StructureSystemTypeId(u16);

impl From<StructureSystemTypeId> for u16 {
    fn from(value: StructureSystemTypeId) -> Self {
        value.0
    }
}

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

#[derive(Component, Debug, Reflect)]
/// Stores all the systems a structure has
pub struct StructureSystems {
    /// These entities should have the `StructureSystem` component
    systems: Vec<StructureSystemId>,
    activatable_systems: Vec<StructureSystemId>,
    /// The system ids
    ids: HashMap<StructureSystemId, Entity>,
    /// More than just one system can be active at a time, but the pilot can only personally activate one system at a time
    /// Perhaps make this a component on the pilot entity in the future?
    /// Currently this limits a ship to one pilot, the above would fix this issue, but this is a future concern.
    active_system: ShipActiveSystem,
    entity: Entity,
}

/// Iterates over structure systems a structure has
pub struct SystemsIterator<'a> {
    iterating_over: &'a [StructureSystemId],
    id_mapping: &'a HashMap<StructureSystemId, Entity>,
}

impl Iterator for SystemsIterator<'_> {
    type Item = Entity;

    fn next(&mut self) -> Option<Self::Item> {
        let system = self.iterating_over.first()?;

        self.iterating_over = &self.iterating_over[1..];

        Some(
            *self
                .id_mapping
                .get(system)
                .expect("Invalid state - system id has no entity mapping"),
        )
    }
}

impl StructureSystems {
    /// Returns an iterator through every system
    pub fn all_systems<'a>(&'a self) -> SystemsIterator<'a> {
        SystemsIterator::<'a> {
            id_mapping: &self.ids,
            iterating_over: &self.systems,
        }
    }

    /// Returns an iterator that only iterates over the activatable systems
    pub fn all_activatable_systems(&self) -> SystemsIterator {
        SystemsIterator {
            id_mapping: &self.ids,
            iterating_over: &self.activatable_systems,
        }
    }

    /// This index is relative to the [`all_systems`] iterator.
    pub fn get_system_from_index(&self, idx: usize) -> Entity {
        *self
            .ids
            .get(&self.systems[idx])
            .expect("Invalid state - system id has no entity mapping")
    }

    /// This index is relative to the [`all_activatable_systems`] iterator NOT the [`all_systems`] index.
    pub fn get_activatable_system_from_activatable_index(&self, idx: usize) -> Entity {
        self.try_get_activatable_system_from_activatable_index(idx)
            .unwrap_or_else(|| panic!("Invalid active system index provided {idx} < {}.", self.ids.len()))
    }

    /// This index is relative to the [`all_activatable_systems`] iterator NOT the [`all_systems`] index.
    pub fn try_get_activatable_system_from_activatable_index(&self, idx: usize) -> Option<Entity> {
        self.activatable_systems
            .get(idx)
            .map(|x| *self.ids.get(x).expect("Invalid state - system id has no entity mapping"))
    }

    fn has_id(&self, id: StructureSystemId) -> bool {
        self.ids.contains_key(&id)
    }

    fn insert_system(&mut self, system_id: StructureSystemId, system_type: &StructureSystemType, entity: Entity) {
        self.ids.insert(system_id, entity);
        self.systems.push(system_id);
        // This ensures the client + server have the same order, which is important.
        // Making this up to user preference would be pointless, since they only should be able
        // to interact with activatable systems.
        self.systems.sort();
        if system_type.is_activatable() {
            self.activatable_systems.push(system_id);
            // This ensures the client + server have the same order, which is important.
            // In the future, this should be up to user-preference.
            self.activatable_systems.sort();
        }
    }

    /// Gets the entity that corresponds to the system id, or none if not found.
    pub fn get_system_entity(&self, system_id: StructureSystemId) -> Option<Entity> {
        self.ids.get(&system_id).copied()
    }

    /// Activates the passed in selected system, and deactivates the system that was previously selected
    ///
    /// The passed in system index must be based off the [`Self::all_activatable_systems`] iterator.
    pub fn set_active_system(&mut self, active: ShipActiveSystem, commands: &mut Commands) {
        if active == self.active_system {
            return;
        }

        if let ShipActiveSystem::Active(active_system) = self.active_system
            && (active_system as usize) < self.activatable_systems.len() {
                let ent = self
                    .ids
                    .get(&self.activatable_systems[active_system as usize])
                    .expect("Invalid state - system id has no entity mapping");

                commands.entity(*ent).remove::<SystemActive>();
            }

        match active {
            ShipActiveSystem::Active(active_system) => {
                if (active_system as usize) < self.activatable_systems.len() {
                    let ent = self
                        .ids
                        .get(&self.activatable_systems[active_system as usize])
                        .expect("Invalid state - system id has no entity mapping");

                    commands.entity(*ent).insert(SystemActive);

                    self.active_system = active;
                } else {
                    self.active_system = ShipActiveSystem::None;
                }
            }
            ShipActiveSystem::Hovered(hovered_system) => {
                if (hovered_system as usize) < self.activatable_systems.len() {
                    self.active_system = active;
                } else {
                    self.active_system = ShipActiveSystem::None;
                }
            }
            ShipActiveSystem::None => self.active_system = ShipActiveSystem::None,
        }
    }

    /// Returns the active system entity, if there is one.
    pub fn active_system(&self) -> Option<Entity> {
        match self.active_system {
            ShipActiveSystem::Active(active_system_idx) => Some(
                *self
                    .ids
                    .get(&self.activatable_systems[active_system_idx as usize])
                    .expect("Invalid state - system id has no entity mapping"),
            ),
            _ => None,
        }
    }

    /// Returns the hovered system entity, if there is one.
    ///
    /// If this system is active, it would still also count as hovered.
    pub fn hovered_system(&self) -> Option<Entity> {
        match self.active_system {
            ShipActiveSystem::Active(active_system_idx) | ShipActiveSystem::Hovered(active_system_idx) => Some(
                *self
                    .ids
                    .get(&self.activatable_systems[active_system_idx as usize])
                    .expect("Invalid state - system id has no entity mapping"),
            ),
            ShipActiveSystem::None => None,
        }
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
    ///
    /// If you don't know what the id should be, use [`Self::add_system`]
    pub fn add_system_with_id<T: StructureSystemImpl>(
        &mut self,
        commands: &mut Commands,
        system: T,
        system_id: StructureSystemId,
        registry: &Registry<StructureSystemType>,
    ) -> Entity {
        let mut ent = None;

        commands.entity(self.entity).with_children(|p| {
            let Some(system_type) = registry.from_id(T::unlocalized_name()) else {
                return;
            };

            let entity = p
                .spawn(system)
                .insert(StructureSystem {
                    structure_entity: self.entity,
                    system_id,
                    system_type_id: system_type.id,
                })
                .id();

            self.insert_system(system_id, system_type, entity);

            ent = Some(entity);
        });

        ent.expect("This should have been set in above closure.")
    }

    /// Adds a system to the structure. Use this instead of directly adding it with commands.
    pub fn add_system<T: StructureSystemImpl>(
        &mut self,
        commands: &mut Commands,
        system: T,
        registry: &Registry<StructureSystemType>,
    ) -> Entity {
        let system_id = self.generate_new_system_id();

        self.add_system_with_id(commands, system, system_id, registry)
    }

    /// Queries all the systems of a structure with this specific query, or returns `Err(NoSystemFound)` if none matched this query.
    pub fn query<'a, Q, F>(&'a self, query: &'a Query<Q, F>) -> Result<ROQueryItem<'a, Q>, NoSystemFound>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        for ent in self
            .systems
            .iter()
            .map(|x| self.ids.get(x).expect("Invalid state - system id has no entity mapping"))
        {
            if let Ok(res) = query.get(*ent) {
                return Ok(res);
            }
        }

        Err(NoSystemFound)
    }

    /// Queries all the systems of a structure with this specific query, or returns `Err(NoSystemFound)` if none matched this query.
    pub fn query_mut<'a, Q, F>(&'a self, query: &'a mut Query<Q, F>) -> Result<QueryItem<'a, Q>, NoSystemFound>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        for ent in self
            .systems
            .iter()
            .map(|x| self.ids.get(x).expect("Invalid state - system id has no entity mapping"))
        {
            // the borrow checker gets mad when I do a get_mut in this if statement
            if query.contains(*ent) {
                return Ok(query.get_mut(*ent).expect("This should be valid"));
            }
        }

        Err(NoSystemFound)
    }
}

fn add_structure(mut commands: Commands, query: Query<Entity, (Added<Structure>, With<Ship>)>) {
    for entity in query.iter() {
        commands.entity(entity).insert(StructureSystems {
            systems: Vec::new(),
            activatable_systems: Vec::new(),
            entity,
            active_system: ShipActiveSystem::None,
            ids: Default::default(),
        });
    }
}

/// A structure system should implement this
pub trait StructureSystemImpl: Component + std::fmt::Debug {
    /// The unlocalized name of this system. Used for unique serialization
    fn unlocalized_name() -> &'static str;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Links a structure system's type id with their unlocalized name
pub struct StructureSystemType {
    unlocalized_name: String,
    id: StructureSystemTypeId,

    activatable: bool,
    item_icon: u16,
}

impl StructureSystemType {
    /// Creates a new structure system type
    pub fn new(unlocalized_name: impl Into<String>, activatable: bool, item_icon: u16) -> Self {
        Self {
            id: StructureSystemTypeId::default(),
            unlocalized_name: unlocalized_name.into(),
            activatable,
            item_icon,
        }
    }

    /// The numeric id of this structure system type
    pub fn system_type_id(&self) -> StructureSystemTypeId {
        self.id
    }

    /// Returns the item icon for this structure system. This is guarenteed to be a valid item's id
    pub fn item_icon_id(&self) -> u16 {
        self.item_icon
    }

    /// Returns true if this system can be activated by the pilot
    pub fn is_activatable(&self) -> bool {
        self.activatable
    }
}

impl Identifiable for StructureSystemType {
    fn id(&self) -> u16 {
        self.id.0
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = StructureSystemTypeId(id);
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<StructureSystemType>(app, "cosmos:structure_system_types");
    sync_registry::<StructureSystemType>(app);

    app.configure_sets(
        Update,
        (
            StructureSystemsSet::InitSystems.in_set(StructureLoadingSet::StructureLoaded),
            StructureSystemsSet::UpdateSystemsBlocks,
            StructureSystemsSet::UpdateSystems,
        )
            .chain(),
    );

    app.add_systems(
        Update,
        (
            add_structure.in_set(StructureLoadingSet::LoadChunkData),
            remove_system_actives_when_melting_down.in_set(StructureSystemsSet::UpdateSystems),
        ),
    )
    .register_type::<StructureSystem>()
    .register_type::<StructureSystems>();

    line_system::register(app);
    shield_system::register(app);
    camera_system::register(app);
    energy_storage_system::register(app);
    energy_generation_system::register(app);
    thruster_system::register(app);
    missile_launcher_system::register(app);
    laser_cannon_system::register(app);
    mining_laser_system::register(app);
    dock_system::register(app);
    railgun_system::register(app);
}
