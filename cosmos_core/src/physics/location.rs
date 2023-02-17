use std::ops::Add;

use bevy::{
    prelude::{App, Changed, Component, DetectChanges, Query, Transform, Vec3},
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

/// This represents the diameter of a sector. So at a local
/// of 0, 0, 0 you can travel `SECTOR_DIMENSIONS / 2.0` blocks in any direction and
/// remain within it.
pub const SECTOR_DIMENSIONS: f32 = 5_000.0;

#[derive(
    Default, Component, Debug, PartialEq, Serialize, Deserialize, Reflect, FromReflect, Clone, Copy,
)]
pub struct Location {
    pub local: Vec3,

    pub sector_x: i64,
    pub sector_y: i64,
    pub sector_z: i64,
}

impl Add<Vec3> for Location {
    type Output = Location;

    fn add(self, rhs: Vec3) -> Self::Output {
        Location::new(
            self.local + rhs,
            self.sector_x,
            self.sector_y,
            self.sector_z,
        )
    }
}

impl Location {
    pub fn new(local: Vec3, sector_x: i64, sector_y: i64, sector_z: i64) -> Self {
        Self {
            local,
            sector_x,
            sector_y,
            sector_z,
        }
    }

    pub fn relativie_coords_to(&self, other: &Location) -> Vec3 {
        let (dsx, dsy, dsz) = (
            (other.sector_x - self.sector_x) as f32,
            (other.sector_y - self.sector_y) as f32,
            (other.sector_z - self.sector_z) as f32,
        );

        Vec3::new(
            SECTOR_DIMENSIONS * dsx + (other.local.x - self.local.x),
            SECTOR_DIMENSIONS * dsy + (other.local.y - self.local.y),
            SECTOR_DIMENSIONS * dsz + (other.local.z - self.local.z),
        )
    }

    pub fn set_from(&mut self, other: &Location) {
        self.local = other.local;
        self.sector_x = other.sector_x;
        self.sector_y = other.sector_y;
        self.sector_z = other.sector_z;
    }
}

fn sync_locations(mut query: Query<(&Transform, &mut Location), Changed<Transform>>) {
    for (trans, mut loc) in query.iter_mut() {
        // Really not that great, but I can't think of any other way of avoiding recursively changing each other
        loc.bypass_change_detection().local = trans.translation;
    }
}

/// This has to be put after specific systems in the server/client or jitter happens
pub fn sync_translations(mut query: Query<(&mut Transform, &Location), Changed<Location>>) {
    for (mut trans, loc) in query.iter_mut() {
        // Really not that great, but I can't think of any other way of avoiding recursively changing each other
        trans.bypass_change_detection().translation = loc.local;
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<Location>().add_system(sync_locations);
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec3;

    use crate::physics::location::SECTOR_DIMENSIONS;

    use super::Location;

    #[test]
    fn in_same_sector_pos() {
        let l1 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);

        let result = Vec3::new(30.0, 30.0, 30.0);

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }

    #[test]
    fn in_same_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 20, -20, 20);

        let result = Vec3::new(-30.0, -30.0, -30.0);

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }

    #[test]
    fn in_diff_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 19, -21, 19);

        let result = Vec3::new(
            -30.0 - SECTOR_DIMENSIONS,
            -30.0 - SECTOR_DIMENSIONS,
            -30.0 - SECTOR_DIMENSIONS,
        );

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }

    #[test]
    fn in_diff_sector_pos() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 21, -19, 21);

        let result = Vec3::new(
            -30.0 + SECTOR_DIMENSIONS,
            -30.0 + SECTOR_DIMENSIONS,
            -30.0 + SECTOR_DIMENSIONS,
        );

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }

    #[test]
    fn in_far_sector_pos() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 30, -10, 30);

        let result = Vec3::new(
            -30.0 + SECTOR_DIMENSIONS * 10.0,
            -30.0 + SECTOR_DIMENSIONS * 10.0,
            -30.0 + SECTOR_DIMENSIONS * 10.0,
        );

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }

    #[test]
    fn in_far_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 10, -30, 10);

        let result = Vec3::new(
            -30.0 - SECTOR_DIMENSIONS * 10.0,
            -30.0 - SECTOR_DIMENSIONS * 10.0,
            -30.0 - SECTOR_DIMENSIONS * 10.0,
        );

        assert_eq!(l1.relativie_coords_to(&l2), result);
    }
}
