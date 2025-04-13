//! Shared health logic

use bevy::prelude::*;
use derive_more::derive::Display;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::{sync_component, IdentifiableComponent, SyncableComponent},
    structure::ship::pilot::Pilot,
};

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Display)]
/// This entity is dead
pub struct Dead;

impl IdentifiableComponent for Dead {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:dead"
    }
}

impl SyncableComponent for Dead {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
/// Once this hits 0, the entity will get the [`Dead`] component.
///
/// Use the [`HealthSet`] when using this field.
pub struct Health(u32);

impl IdentifiableComponent for Health {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:health"
    }
}

impl SyncableComponent for Health {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl Health {
    /// Creates a health with this starting value.
    ///
    /// This is NOT checked for not exceeding this entity's [`MaxHealth`]. Make sure to do that
    /// manually.
    pub fn new(starting_value: u32) -> Self {
        Self(starting_value)
    }

    /// The health is reduce by this amount, clamped to 0.
    pub fn take_damage(&mut self, amount: u32) {
        self.0 -= amount.min(self.0);
    }

    /// Checks if this entity should be marked with the [`Dead`] component.
    pub fn is_alive(&self) -> bool {
        self.0 != 0
    }

    /// Heals this entity by the speficied amount, clamped to not exceed [`MaxHealth`]. Note that
    /// this will NOT revive an entity that has previously been given the [`Dead`] component.
    pub fn heal(&mut self, amount: u32, max_health: &MaxHealth) {
        self.0 = (self.0 + amount).min(max_health.0);
    }
}

impl From<Health> for u32 {
    fn from(value: Health) -> Self {
        value.0
    }
}

impl From<MaxHealth> for Health {
    fn from(value: MaxHealth) -> Self {
        Self::new(value.0)
    }
}

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Display)]
/// Represents the maximum [`Health`] value an entity can have.
pub struct MaxHealth(u32);

impl IdentifiableComponent for MaxHealth {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:max_health"
    }
}

impl SyncableComponent for MaxHealth {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl MaxHealth {
    /// Creates a new maximum health value
    pub fn new(value: u32) -> Self {
        Self(value)
    }
}

impl From<MaxHealth> for u32 {
    fn from(value: MaxHealth) -> Self {
        value.0
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Sets handling health should do so relative to this
pub enum HealthSet {
    /// Health changes are handled, such as triggering player death
    ProcessHealthChange,
}

fn on_die(mut commands: Commands, q_dead: Query<Entity, Added<Dead>>) {
    for e in q_dead.iter() {
        commands.entity(e).remove_parent_in_place().remove::<Pilot>();
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Health>(app);
    sync_component::<MaxHealth>(app);
    sync_component::<Dead>(app);

    app.configure_sets(Update, HealthSet::ProcessHealthChange);

    app.register_type::<Health>().register_type::<MaxHealth>();
}
