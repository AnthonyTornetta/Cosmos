use bevy::prelude::*;
use derive_more::derive::Display;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, IdentifiableComponent, SyncableComponent};

#[derive(Component, Serialize, Reflect, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Display)]
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
    pub fn new(starting_value: u32) -> Self {
        Self(starting_value)
    }

    pub fn take_damage(&mut self, amount: u32) {
        self.0 -= amount.min(self.0);
    }

    pub fn is_alive(&self) -> bool {
        self.0 != 0
    }

    pub fn heal(&mut self, amount: u32, max_health: &MaxHealth) {
        self.0 = (self.0 + amount).min(max_health.0);
    }

    pub fn health_percent(&self, max_health: &MaxHealth) -> f32 {
        self.0 as f32 / max_health.0 as f32
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
pub enum HealthSet {
    ProcessHealthChange,
}

pub(super) fn register(app: &mut App) {
    sync_component::<Health>(app);
    sync_component::<MaxHealth>(app);
    sync_component::<Dead>(app);

    app.configure_sets(Update, HealthSet::ProcessHealthChange);

    app.register_type::<Health>().register_type::<MaxHealth>();
}
