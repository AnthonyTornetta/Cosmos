use bevy::{
    prelude::{App, Commands, Component, CoreSet, Entity, IntoSystemConfig, Query, Without},
    reflect::{FromReflect, Reflect},
};

use crate::physics::location::SECTOR_DIMENSIONS;

pub const LOAD_DISTANCE: f32 = SECTOR_DIMENSIONS * 8.0;

#[derive(Component, Debug, Reflect, FromReflect, Clone, Copy)]
/// Use this to have a custom distance for something to be unloaded.
///
/// This distance is in # of sectors. The default is 10.
pub struct UnloadDistance {
    load_distance: u32,
    unload_distance: u32,
}

impl Default for UnloadDistance {
    fn default() -> Self {
        Self::new(8, 10)
    }
}

impl UnloadDistance {
    pub fn new(load_distance: u32, unload_distance: u32) -> Self {
        Self {
            load_distance,
            unload_distance,
        }
    }

    #[inline]
    pub fn unload_distance(&self) -> u32 {
        self.unload_distance
    }

    #[inline]
    pub fn load_distance(&self) -> u32 {
        self.load_distance
    }

    #[inline]
    pub fn load_block_distance(&self) -> f32 {
        self.load_distance as f32 * SECTOR_DIMENSIONS
    }

    #[inline]
    pub fn unload_block_distance(&self) -> f32 {
        self.unload_distance as f32 * SECTOR_DIMENSIONS
    }
}

fn add_unload_distance(query: Query<Entity, Without<UnloadDistance>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert(UnloadDistance::default());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_unload_distance.in_base_set(CoreSet::First))
        .register_type::<UnloadDistance>();
}
