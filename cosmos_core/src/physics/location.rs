use std::ops::{Add, AddAssign};

use bevy::{
    prelude::{App, Children, Component, Entity, Parent, Query, Transform, Vec3, With, Without},
    reflect::{FromReflect, Reflect},
};
use serde::{Deserialize, Serialize};

/// This represents the diameter of a sector. So at a local
/// of 0, 0, 0 you can travel `SECTOR_DIMENSIONS / 2.0` blocks in any direction and
/// remain within it.
pub const SECTOR_DIMENSIONS: f32 = 10_000.0;

#[derive(
    Default, Component, Debug, PartialEq, Serialize, Deserialize, Reflect, FromReflect, Clone, Copy,
)]
pub struct Location {
    pub local: Vec3,

    pub sector_x: i64,
    pub sector_y: i64,
    pub sector_z: i64,

    pub last_transform_loc: Vec3,
}

impl Add<Vec3> for Location {
    type Output = Location;

    fn add(self, rhs: Vec3) -> Self::Output {
        let mut loc = Location::new(
            self.local + rhs,
            self.sector_x,
            self.sector_y,
            self.sector_z,
        );
        loc.fix_bounds();
        loc
    }
}

impl AddAssign<Vec3> for &mut Location {
    fn add_assign(&mut self, rhs: Vec3) {
        self.local += rhs;
        self.fix_bounds();
    }
}

impl Location {
    pub fn new(local: Vec3, sector_x: i64, sector_y: i64, sector_z: i64) -> Self {
        Self {
            local,
            sector_x,
            sector_y,
            sector_z,
            last_transform_loc: local,
        }
    }

    pub fn fix_bounds(&mut self) {
        let over_x = (self.local.x / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_x != 0 {
            self.local.x -= over_x as f32 * SECTOR_DIMENSIONS;
            self.sector_x += over_x as i64;
        }

        let over_y = (self.local.y / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_y != 0 {
            self.local.y -= over_y as f32 * SECTOR_DIMENSIONS;
            self.sector_y += over_y as i64;
        }

        let over_z = (self.local.z / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_z != 0 {
            self.local.z -= over_z as f32 * SECTOR_DIMENSIONS;
            self.sector_z += over_z;
        }
    }

    pub fn relative_coords_to(&self, other: &Location) -> Vec3 {
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

    pub fn apply_updates(&mut self, translation: Vec3) {
        self.local += translation - self.last_transform_loc;
        self.fix_bounds();

        self.last_transform_loc = translation;
    }
}

fn bubble(
    loc: &Location,
    entity: Entity,
    mut query: &mut Query<(&mut Location, &Transform, Option<&Children>), With<Parent>>,
) {
    let mut todos = Vec::new();

    if let Ok((mut location, transform, children)) = query.get_mut(entity) {
        location.set_from(loc);
        location.local += transform.translation;
        location.last_transform_loc = transform.translation;
        location.fix_bounds();

        if let Some(children) = children {
            for child in children {
                todos.push((*child, *location));
            }
        }
    }

    for (entity, loc) in todos {
        bubble(&loc, entity, &mut query);
    }
}

/// Makes sure children have proper locations, this should be added after syncing transforms & locations.
pub fn bubble_down_locations(
    tops: Query<(&Location, &Children), Without<Parent>>,
    mut middles: Query<(&mut Location, &Transform, Option<&Children>), With<Parent>>,
) {
    for (loc, children) in tops.iter() {
        for entity in children.iter() {
            bubble(loc, *entity, &mut middles);
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.register_type::<Location>();
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

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_same_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), 20, -20, 20);
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), 20, -20, 20);

        let result = Vec3::new(-30.0, -30.0, -30.0);

        assert_eq!(l1.relative_coords_to(&l2), result);
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

        assert_eq!(l1.relative_coords_to(&l2), result);
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

        assert_eq!(l1.relative_coords_to(&l2), result);
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

        assert_eq!(l1.relative_coords_to(&l2), result);
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

        assert_eq!(l1.relative_coords_to(&l2), result);
    }
}
