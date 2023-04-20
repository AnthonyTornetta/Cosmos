//! Responsible for shared saving/loading logic

use bevy::{
    prelude::{App, Commands, Component, CoreSet, Entity, IntoSystemConfig, Query, Without},
    reflect::{FromReflect, Reflect},
};

use crate::physics::location::SECTOR_DIMENSIONS;

/// The default loading distance for structures
pub const LOAD_DISTANCE: f32 = SECTOR_DIMENSIONS * 8.0;

#[derive(Component, Debug, Reflect, FromReflect, Clone, Copy)]
/// Use this to have a custom distance for something to be unloaded.
///
/// This distance is in # of sectors. The default is 10.
pub struct LoadingDistance {
    load_distance: u32,
    unload_distance: u32,
}

impl Default for LoadingDistance {
    fn default() -> Self {
        Self::new(8, 10)
    }
}

impl LoadingDistance {
    /// Creates a new loading distance
    ///
    /// * `load_distance` This is how far away something has to be to be loaded. This must be < `unload_distance`. An assertion assures this
    /// * `unload_distance` This is how far away something has to be to be unloaded
    pub fn new(load_distance: u32, unload_distance: u32) -> Self {
        assert!(load_distance <= unload_distance);
        Self {
            load_distance,
            unload_distance,
        }
    }

    #[inline]
    /// Gets the distance where something should be unloaded in sectors
    pub fn unload_distance(&self) -> u32 {
        self.unload_distance
    }

    #[inline]
    /// Gets the distance where something should be loaded in sectors
    pub fn load_distance(&self) -> u32 {
        self.load_distance
    }

    #[inline]
    /// Gets the distance where something should be loaded in blocks
    pub fn load_block_distance(&self) -> f32 {
        self.load_distance as f32 * SECTOR_DIMENSIONS
    }

    #[inline]

    /// Gets the distance where something should be unloaded in blocks
    pub fn unload_block_distance(&self) -> f32 {
        self.unload_distance as f32 * SECTOR_DIMENSIONS
    }
}

fn add_unload_distance(query: Query<Entity, Without<LoadingDistance>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert(LoadingDistance::default());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_unload_distance.in_base_set(CoreSet::First))
        .register_type::<LoadingDistance>();
}
