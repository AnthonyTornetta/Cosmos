//! Represents a planet's atmosphere

use bevy::{
    color::Color,
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, IdentifiableComponent, SyncableComponent};

#[derive(PartialEq, Debug, Clone, Copy, Reflect, Component, Serialize, Deserialize)]
/// Represents the details about a planet's atmosphere.
///
/// Currently just for rendering by the client, may be more in the future.
pub struct PlanetAtmosphere(Color);

impl PlanetAtmosphere {
    /// Creates a new atmosphere based on this color.
    pub fn new(color: Color) -> Self {
        Self(color)
    }

    /// Returns the color this atmosphere should be
    pub fn color(&self) -> &Color {
        &self.0
    }
}

impl IdentifiableComponent for PlanetAtmosphere {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:atmosphere_color"
    }
}

impl SyncableComponent for PlanetAtmosphere {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<PlanetAtmosphere>(app);

    app.register_type::<PlanetAtmosphere>();
}
