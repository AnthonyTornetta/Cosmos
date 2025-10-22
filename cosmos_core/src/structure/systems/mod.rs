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
    platform::collections::HashMap,
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::{NeedsDespawned, data::DataFor},
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
        registry::sync_registry,
        sync_component,
    },
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
pub mod warp;

#[derive(Component, Debug, Reflect, PartialEq, Eq, Serialize, Deserialize, Clone, Copy)]
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
pub enum SystemActive {
    /// The primary function of this system was used (left click)
    Primary,
    /// The secondary function of this system was used (right click)
    Secondary,
    /// Both functions of this system were used (left + right click)
    Both,
}

impl SystemActive {
    /// Returns true if the Priamry or Both systems are being used
    pub fn primary(&self) -> bool {
        matches!(self, Self::Primary | Self::Both)
    }

    /// Returns true if the Secondary or Both systems are being used
    pub fn secondary(&self) -> bool {
        matches!(self, Self::Secondary | Self::Both)
    }
}

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

#[derive(Component, Debug, Reflect, Clone, Copy)]
/// Every system has this as a component.
pub struct StructureSystem {
    structure_entity: Entity,
    system_id: StructureSystemId,
    system_type_id: StructureSystemTypeId,
}

impl IdentifiableComponent for StructureSystem {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_system"
    }
}

impl StructureSystem {
    /// Creates a structure system from raw data. This should only be used if you are loading this
    /// from serialized data.
    pub fn from_raw(structure_entity: Entity, system_id: StructureSystemId, system_type_id: StructureSystemTypeId) -> Self {
        Self {
            structure_entity,
            system_type_id,
            system_id,
        }
    }

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

#[derive(Event, Serialize, Deserialize, Debug, Clone)]
/// Sent by the player to request changing a system slot to point to a specific system
pub struct ChangeSystemSlot {
    /// The system they want the slot to be (or `None` to clear it)
    pub system_id: Option<StructureSystemId>,
    /// The structure that are changging (must be the one they are piloting - leaving this field
    /// for now in case I add other conditions later)
    pub structure: Entity,
    /// 0-8
    pub slot: u32,
}

impl IdentifiableEvent for ChangeSystemSlot {
    fn unlocalized_name() -> &'static str {
        "cosmos:change_system_slot"
    }
}

impl NettyEvent for ChangeSystemSlot {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.structure).map(|e| Self {
            structure: e,
            system_id: self.system_id,
            slot: self.slot,
        })
    }
}

#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Eq, Reflect)]
/// Represents the ordering of activatable [`StructureSystem`]s that can be directly activated by
/// the player.
pub struct StructureSystemOrdering {
    // 0-8
    system_slots: Vec<Option<StructureSystemId>>,
}

impl Default for StructureSystemOrdering {
    fn default() -> Self {
        Self {
            system_slots: vec![None; 9],
        }
    }
}

impl IdentifiableComponent for StructureSystemOrdering {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_system_ordering"
    }
}

impl SyncableComponent for StructureSystemOrdering {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl StructureSystemOrdering {
    /// Sets the slot for this system to be activated from
    pub fn set_slot(&mut self, slot: u32, system: StructureSystemId) {
        if slot < self.system_slots.len() as u32 {
            self.system_slots[slot as usize] = Some(system);
        } else {
            error!("Invalid set slot - {slot}");
        }
    }

    /// This slot will no longer point to a system
    pub fn clear_slot(&mut self, slot: u32) {
        if slot < self.system_slots.len() as u32 {
            self.system_slots[slot as usize] = None;
        } else {
            error!("Invalid clear slot - {slot}");
        }
    }

    /// Returns the system at this slot
    pub fn get_slot(&self, slot: u32) -> Option<StructureSystemId> {
        if slot < self.system_slots.len() as u32 {
            self.system_slots[slot as usize]
        } else {
            error!("Invalid get slot - {slot}");
            None
        }
    }

    /// Iterates over every slot that can be used by the player - even those that contain no
    /// systems
    pub fn iter(&self) -> impl Iterator<Item = Option<StructureSystemId>> {
        self.system_slots.iter().copied()
    }

    /// Returns the slot this system is in (if it is in any slot)
    pub fn ordering_for(&self, system_id: StructureSystemId) -> Option<u32> {
        self.iter().enumerate().find(|(_, x)| *x == Some(system_id)).map(|x| x.0 as u32)
    }

    /// Adds this system to the next available slot, or does nothing if no slots are available.
    pub fn add_to_next_available(&mut self, system_id: StructureSystemId) {
        if let Some(slot) = self.system_slots.iter_mut().find(|x| x.is_none()) {
            *slot = Some(system_id);
        }
    }
}

#[derive(Component, Debug, Reflect)]
#[require(StructureSystemOrdering)]
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

impl StructureSystems {
    /// For saving this to disk
    pub fn activatable_systems(&self) -> &[StructureSystemId] {
        self.activatable_systems.as_slice()
    }
    /// For saving this to disk
    pub fn systems(&self) -> &[StructureSystemId] {
        self.systems.as_slice()
    }
    /// For saving this to disk
    pub fn ids(&self) -> &HashMap<StructureSystemId, Entity> {
        &self.ids
    }
    /// WARNING: Only call this if you know what you're doing!
    ///
    /// This needs to be properly initialized after this is called with its own entity via
    /// [`Self::set_entity`]. This should really only be used for deserialization.
    pub fn new_from_raw(
        systems: Vec<StructureSystemId>,
        activatable_systems: Vec<StructureSystemId>,
        ids: HashMap<StructureSystemId, Entity>,
    ) -> Self {
        Self {
            activatable_systems,
            ids,
            active_system: ShipActiveSystem::None,
            systems,
            // This will get immediately set when this is initialized
            entity: Entity::PLACEHOLDER,
        }
    }

    /// Sets the self entity of this structure system - this should be the Structure it's apart of.
    pub fn set_entity(&mut self, entity: Entity) {
        self.entity = entity;
    }
}

impl IdentifiableComponent for StructureSystems {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_systems"
    }
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
    /// The passed in system index must be based off the [`StructureSystemOrdering`] ordering.
    pub fn set_active_system(
        &mut self,
        active: ShipActiveSystem,
        ordering: &StructureSystemOrdering,
        commands: &mut Commands,
        type_of_active: Option<SystemActive>,
    ) {
        if active == self.active_system {
            return;
        }

        if let Some(ent) = self.active_system(ordering) {
            commands.entity(ent).remove::<SystemActive>();
        }

        self.active_system = active;

        if let Some(type_of_active) = type_of_active {
            if let Some(ent) = self.active_system(ordering) {
                commands.entity(ent).insert(type_of_active);

                self.active_system = active;
            } else if self.hovered_system(ordering).is_none() {
                self.active_system = ShipActiveSystem::None;
            }
        }
    }

    /// Returns the active system entity, if there is one.
    pub fn active_system(&self, ordering: &StructureSystemOrdering) -> Option<Entity> {
        match self.active_system {
            ShipActiveSystem::Active(active_system_idx) => ordering
                .get_slot(active_system_idx)
                .map(|x| *self.ids.get(&x).expect("Invalid state - system id has no entity mapping")),
            _ => None,
        }
    }

    /// Returns the hovered system entity, if there is one.
    ///
    /// If this system is active, it would still also count as hovered.
    pub fn hovered_system(&self, ordering: &StructureSystemOrdering) -> Option<Entity> {
        match self.active_system {
            ShipActiveSystem::Active(active_system_idx) | ShipActiveSystem::Hovered(active_system_idx) => {
                ordering.get_slot(active_system_idx).map(|x| {
                    info!("{x:?}");
                    *self.ids.get(&x).expect("Invalid state - system id has no entity mapping")
                })
            }
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
                .spawn((
                    system,
                    DataFor(self.entity),
                    StructureSystem {
                        structure_entity: self.entity,
                        system_id,
                        system_type_id: system_type.id,
                    },
                ))
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
    ) -> (StructureSystemId, Entity) {
        let system_id = self.generate_new_system_id();

        (system_id, self.add_system_with_id(commands, system, system_id, registry))
    }

    /// Removes a structure system from this systems registry. This will also despawn the system
    /// and its children.
    pub fn remove_system(
        &mut self,
        commands: &mut Commands,
        system: &StructureSystem,
        registry: &Registry<StructureSystemType>,
        ordering: &mut StructureSystemOrdering,
    ) {
        let system_id = system.system_id;
        if let Some(slot) = ordering.ordering_for(system_id) {
            ordering.clear_slot(slot);
        }

        let Some(entity) = self.ids.remove(&system_id) else {
            return;
        };
        commands.entity(entity).insert(NeedsDespawned);

        let Some((idx, _)) = self.systems.iter().enumerate().find(|(_, x)| **x == system_id) else {
            return;
        };
        self.systems.remove(idx);
        // This ensures the client + server have the same order, which is important.
        // Making this up to user preference would be pointless, since they only should be able
        // to interact with activatable systems.
        self.systems.sort();
        if registry.from_numeric_id(system.system_type_id.0).is_activatable() {
            let Some((idx, _)) = self.activatable_systems.iter().enumerate().find(|(_, x)| **x == system_id) else {
                return;
            };

            self.activatable_systems.remove(idx);
            // This ensures the client + server have the same order, which is important.
            // In the future, this should be up to user-preference.
            self.activatable_systems.sort();
        }
    }

    /// Queries all the systems of a structure with this specific query, or returns `Err(NoSystemFound)` if none matched this query.
    pub fn query<'a, Q, F>(&'a self, query: &'a Query<Q, F>) -> Result<ROQueryItem<'a, Q>, NoSystemFound>
    where
        F: QueryFilter,
        Q: QueryData,
    {
        for ent in self.systems.iter().flat_map(|x| self.ids.get(x)) {
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
        for ent in self.systems.iter().flat_map(|x| self.ids.get(x)) {
            // the borrow checker gets mad when I do a get_mut in this if statement
            if query.contains(*ent) {
                return Ok(query.get_mut(*ent).expect("This should be valid"));
            }
        }

        Err(NoSystemFound)
    }
}

fn add_structure(mut commands: Commands, query: Query<Entity, (Added<Structure>, Without<StructureSystems>, With<Ship>)>) {
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

impl<T: StructureSystemImpl> IdentifiableComponent for T {
    fn get_component_unlocalized_name() -> &'static str {
        Self::unlocalized_name()
    }
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

/// Used to illustrate charge to client - not used for any logic and is not automatically managed
///
/// The value should be bounded between 0.0 to 1.0
#[derive(Component, Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct StructureSystemCharge(pub f32);

impl IdentifiableComponent for StructureSystemCharge {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_system_charge"
    }
}

impl SyncableComponent for StructureSystemCharge {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    create_registry::<StructureSystemType>(app, "cosmos:structure_system_types");
    sync_registry::<StructureSystemType>(app);

    sync_component::<StructureSystemOrdering>(app);
    sync_component::<StructureSystemCharge>(app);

    app.configure_sets(
        FixedUpdate,
        (
            StructureSystemsSet::InitSystems,
            StructureSystemsSet::UpdateSystemsBlocks,
            StructureSystemsSet::UpdateSystems,
        )
            .chain(),
    );

    app.add_systems(
        FixedUpdate,
        (
            add_structure.in_set(StructureLoadingSet::LoadChunkData),
            remove_system_actives_when_melting_down.in_set(StructureSystemsSet::UpdateSystems),
        ),
    )
    .register_type::<StructureSystem>()
    .register_type::<StructureSystems>()
    .register_type::<StructureSystemOrdering>()
    .register_type::<SystemActive>()
    .add_netty_event::<ChangeSystemSlot>();

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
    warp::register(app);
}
