//! Used to identify the causer of this projectile.
//!
//! Typically used to determine damage source

use bevy::{
    prelude::{App, Component, Entity},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "client")]
use crate::netty::sync::mapping::Mappable;
#[cfg(feature = "client")]
use crate::netty::sync::mapping::MappingError;

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize)]
/// Identifies who fired this projectile.
pub struct Causer(pub Entity);

#[cfg(feature = "client")]
impl Mappable for Causer {
    fn map(
        self,
        network_mapping: &crate::netty::sync::mapping::NetworkMapping,
    ) -> Result<Self, crate::netty::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        network_mapping
            .client_from_server(&self.0)
            .map(|x| Ok(Self(x)))
            .unwrap_or(Err(MappingError::MissingRecord(self)))
    }

    fn map_to_server(
        self,
        network_mapping: &crate::netty::sync::mapping::NetworkMapping,
    ) -> Result<Self, crate::netty::sync::mapping::MappingError<Self>>
    where
        Self: Sized,
    {
        network_mapping
            .server_from_client(&self.0)
            .map(|x| Ok(Self(x)))
            .unwrap_or(Err(MappingError::MissingRecord(self)))
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Causer>();
}
