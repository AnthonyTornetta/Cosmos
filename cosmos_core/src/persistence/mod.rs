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
pub struct CustomUnloadDistance(u32);

impl Default for CustomUnloadDistance {
    fn default() -> Self {
        Self(10)
    }
}

impl CustomUnloadDistance {
    pub fn new(distance: u32) -> Self {
        Self(distance)
    }

    #[inline]
    pub fn distance(&self) -> u32 {
        self.0
    }

    #[inline]
    pub fn block_distance_squared(&self) -> f32 {
        (self.distance() * self.distance()) as f32 * SECTOR_DIMENSIONS * SECTOR_DIMENSIONS
    }
}

fn add_unload_distance(
    query: Query<Entity, Without<CustomUnloadDistance>>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(CustomUnloadDistance::default());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_unload_distance.in_base_set(CoreSet::First))
        .register_type::<CustomUnloadDistance>();
}
