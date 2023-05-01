//! Used to represent a point in a near-infinite space
//!
//! Rather than represent coordinates as imprecise f32, a location is used instead.
//! A location contains sector coordinates (i64) and local coordinates (f32).
//!
//! The local coordinates are bound to be within [`-SECTOR_DIMENSIONS`, `SECTOR_DIMENSIONS`].
//! If they leave this range at any time, the sector coordinates are incremented and decremented accordingly.
//!
//! This allows locations to store near-infinite unique points in space.
//!
//! Due to the physics engine + bevy using Transform (which uses f32s), locations will be updated
//! from changes to transforms. However, to ensure everything works fine all the time, you should prefer
//! to update the location component rather than the Transform where possible.

use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub},
};

use bevy::{
    prelude::{App, Children, Component, Entity, Parent, Query, Transform, Vec3, With, Without},
    reflect::{FromReflect, Reflect},
};
use bevy_rapier3d::na::Vector3;
use bigdecimal::{BigDecimal, FromPrimitive};
use serde::{Deserialize, Serialize};

/// This represents the diameter of a sector. So at a local
/// of 0, 0, 0 you can travel `SECTOR_DIMENSIONS / 2.0` blocks in any direction and
/// remain within it.
pub const SECTOR_DIMENSIONS: f32 = 20_000.0;

/// This represents how many sectors make up one system
pub const SYSTEM_SECTORS: u32 = 100;

/// This is the size in blocks of one system
pub const SYSTEM_DIMENSIONS: f32 = SYSTEM_SECTORS as f32 * SECTOR_DIMENSIONS;

#[derive(
    Default, Component, Debug, PartialEq, Serialize, Deserialize, Reflect, FromReflect, Clone, Copy,
)]
/// Used to represent a point in a near-infinite space
///
/// Rather than represent coordinates as imprecise f32, a location is used instead.
/// A location contains sector coordinates (i64) and local coordinates (f32).
///
/// The local coordinates are bound to be within [`-SECTOR_DIMENSIONS/2.0`, `SECTOR_DIMENSIONS/2.0`].
/// If they leave this range at any time, the sector coordinates are incremented and decremented accordingly.
///
/// This allows locations to store near-infinite unique points in space.
///
/// Due to the physics engine + bevy using Transform (which uses f32s), locations will be updated
/// from changes to transforms. However, to ensure everything works fine all the time, you should prefer
/// to update the location component rather than the Transform where possible.
pub struct Location {
    /// The local coordinates - bounded to be within [`-SECTOR_DIMENSIONS/2.0`, `SECTOR_DIMENSIONS/2.0`]
    pub local: Vec3,

    /// The sector coordinates. One sector unit represents `SECTOR_DIMENSIONS` worth of blocks travelled.
    pub sector_x: i64,
    /// The sector coordinates. One sector unit represents `SECTOR_DIMENSIONS` worth of blocks travelled.
    pub sector_y: i64,
    /// The sector coordinates. One sector unit represents `SECTOR_DIMENSIONS` worth of blocks travelled.
    pub sector_z: i64,

    #[serde(skip)]
    /// Tracks the last transform location. Do not set this unless you know what you're doing.
    ///
    /// This is used to calculate changes in the Transform object & adjust the location accordingly.
    pub last_transform_loc: Option<Vec3>,
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.3}, {:.3}, {:.3}), [{}, {}, {}]",
            self.local.x, self.local.y, self.local.z, self.sector_x, self.sector_y, self.sector_z
        )
    }
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

impl Sub<Vec3> for Location {
    type Output = Location;

    fn sub(self, rhs: Vec3) -> Self::Output {
        let mut loc = Location::new(
            self.local - rhs,
            self.sector_x,
            self.sector_y,
            self.sector_z,
        );
        loc.fix_bounds();
        loc
    }
}

impl Sub<Location> for Location {
    type Output = Location;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut loc = Location::new(
            self.local - rhs.local,
            self.sector_x - rhs.sector_x,
            self.sector_y - rhs.sector_y,
            self.sector_z - rhs.sector_z,
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

impl From<Location> for Vec3 {
    fn from(val: Location) -> Self {
        Vec3::new(
            val.sector_x as f32 * SECTOR_DIMENSIONS + val.local.x,
            val.sector_y as f32 * SECTOR_DIMENSIONS + val.local.y,
            val.sector_z as f32 * SECTOR_DIMENSIONS + val.local.z,
        )
    }
}

impl Location {
    /// Creates a new location at these coordinates
    pub fn new(local: Vec3, sector_x: i64, sector_y: i64, sector_z: i64) -> Self {
        Self {
            local,
            sector_x,
            sector_y,
            sector_z,
            last_transform_loc: Some(local),
        }
    }

    /// Gets the system coordinates this location is in
    pub fn get_system_coordinates(&self) -> (i64, i64, i64) {
        (
            self.sector_x / SYSTEM_SECTORS as i64,
            self.sector_y / SYSTEM_SECTORS as i64,
            self.sector_z / SYSTEM_SECTORS as i64,
        )
    }

    /// Gets the sector coordinates as a tuple
    #[inline]
    pub fn sector(&self) -> (i64, i64, i64) {
        (self.sector_x, self.sector_y, self.sector_z)
    }

    /// Ensures `self.local` is within [`-SECTOR_DIMENSIONS/2.0`, `SECTOR_DIMENSIONS/2.0`]
    ///
    /// If not, the sector coordinates & `local` will be modified to maintain this
    pub fn fix_bounds(&mut self) {
        let over_x = (self.local.x / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_x != 0 {
            self.local.x -= over_x as f32 * SECTOR_DIMENSIONS;
            self.sector_x += over_x;
        }

        let over_y = (self.local.y / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_y != 0 {
            self.local.y -= over_y as f32 * SECTOR_DIMENSIONS;
            self.sector_y += over_y;
        }

        let over_z = (self.local.z / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_z != 0 {
            self.local.z -= over_z as f32 * SECTOR_DIMENSIONS;
            self.sector_z += over_z;
        }
    }

    /// Only usable over f32 distances - will return infinity for distances that are outside the bounds of f32 calculations
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

    /// Only usable over f32 distances - will return infinity for distances that are outside the bounds of f32 calculations
    pub fn distance_sqrd(&self, other: &Location) -> f32 {
        let rel = self.relative_coords_to(other);

        rel.dot(rel)
    }

    /// Sets this from another location.
    ///
    /// Does not update the `last_transform_loc`.
    pub fn set_from(&mut self, other: &Location) {
        self.local = other.local;
        self.sector_x = other.sector_x;
        self.sector_y = other.sector_y;
        self.sector_z = other.sector_z;
    }

    /// Applies updates from the new translation of the transform.
    ///
    /// This is done automatically, so don't worry about it unless you're doing something fancy.
    pub fn apply_updates(&mut self, translation: Vec3) {
        self.local += translation
            - self
                .last_transform_loc
                .expect("last_transform_loc must be set for this to work properly.");
        self.fix_bounds();

        self.last_transform_loc = Some(translation);
    }

    /// Returns the coordinates of this location based off 0, 0, 0.
    ///
    /// Useful for very long-distance calculations/displaying
    pub fn absolute_coords(&self) -> Vector3<BigDecimal> {
        let sector_dims = BigDecimal::from_f32(SECTOR_DIMENSIONS).unwrap();

        let local_x = BigDecimal::from_f32(self.local.x).unwrap();
        let local_y = BigDecimal::from_f32(self.local.y).unwrap();
        let local_z = BigDecimal::from_f32(self.local.z).unwrap();

        Vector3::new(
            BigDecimal::from_i64(self.sector_x).unwrap() * &sector_dims + local_x,
            BigDecimal::from_i64(self.sector_y).unwrap() * &sector_dims + local_y,
            BigDecimal::from_i64(self.sector_z).unwrap() * &sector_dims + local_z,
        )
    }

    /// Returns the coordinates of this location based off 0, 0, 0.
    ///
    /// Useful for short/medium-distance calculations/displaying
    pub fn absolute_coords_f32(&self) -> Vec3 {
        Vec3::new(
            self.sector_x as f32 * SECTOR_DIMENSIONS + self.local.x,
            self.sector_y as f32 * SECTOR_DIMENSIONS + self.local.y,
            self.sector_z as f32 * SECTOR_DIMENSIONS + self.local.z,
        )
    }
}

fn bubble(
    loc: &Location,
    entity: Entity,
    query: &mut Query<(&mut Location, &Transform, Option<&Children>), With<Parent>>,
) {
    let mut todos = Vec::new();

    if let Ok((mut location, transform, children)) = query.get_mut(entity) {
        location.set_from(loc);
        location.local += transform.translation;
        location.last_transform_loc = Some(transform.translation);
        location.fix_bounds();

        if let Some(children) = children {
            for child in children {
                todos.push((*child, *location));
            }
        }
    }

    for (entity, loc) in todos {
        bubble(&loc, entity, query);
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

pub(super) fn register(app: &mut App) {
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
