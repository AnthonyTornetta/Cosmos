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
    prelude::{
        App, Children, Commands, Component, Deref, DerefMut, Entity, GlobalTransform, Parent,
        Query, Transform, Vec3, Without,
    },
    reflect::{FromReflect, Reflect},
};
use bevy_rapier3d::na::Vector3;
use bigdecimal::{BigDecimal, FromPrimitive};
use serde::{Deserialize, Serialize};

use crate::structure::chunk::ChunkEntity;

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
    /// The sector coordinates
    pub sector: Sector,

    #[serde(skip)]
    /// Tracks the last transform location. Do not set this unless you know what you're doing.
    ///
    /// This is used to calculate changes in the Transform object & adjust the location accordingly.
    pub last_transform_loc: Option<Vec3>,
}

/// Datatype used to store sector coordinates
pub type SectorUnit = i64;

#[derive(
    Default,
    Component,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Reflect,
    FromReflect,
    Clone,
    Copy,
    Hash,
    Eq,
)]
/// Represents a large region of space
pub struct Sector(SectorUnit, SectorUnit, SectorUnit);

impl Sector {
    #[inline]
    /// Creates a new Sector at the given coordinates
    pub fn new(x: SectorUnit, y: SectorUnit, z: SectorUnit) -> Self {
        Self(x, y, z)
    }

    #[inline]
    /// sector x
    pub fn x(&self) -> SectorUnit {
        self.0
    }

    #[inline]
    /// sets sector x
    pub fn set_x(&mut self, x: SectorUnit) {
        self.0 = x;
    }

    #[inline]
    /// sector y
    pub fn y(&self) -> SectorUnit {
        self.1
    }

    #[inline]
    /// sets sector y
    pub fn set_y(&mut self, y: SectorUnit) {
        self.1 = y;
    }

    #[inline]
    /// sector z
    pub fn z(&self) -> SectorUnit {
        self.2
    }

    #[inline]
    /// sets sector z
    pub fn set_z(&mut self, z: SectorUnit) {
        self.2 = z;
    }

    /// Computes the absolute value of every coordinate
    pub fn abs(&self) -> Self {
        Self(self.0.abs(), self.1.abs(), self.2.abs())
    }

    /// Computes the maximum element
    pub fn max_element(&self) -> SectorUnit {
        self.0.max(self.1).max(self.2)
    }
}

impl Add<Sector> for Sector {
    type Output = Sector;

    fn add(self, rhs: Sector) -> Self::Output {
        Sector(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

/// Datatype used to store system coordinates
pub type SystemUnit = i64;

#[derive(
    Default, Component, Debug, PartialEq, Serialize, Deserialize, Reflect, FromReflect, Clone, Copy,
)]
/// A universe system represents a large area of sectors
pub struct UniverseSystem(SystemUnit, SystemUnit, SystemUnit);

impl UniverseSystem {
    #[inline]
    /// Creates a new UniverseSystem at the given system coordinates
    pub fn new(x: SystemUnit, y: SystemUnit, z: SystemUnit) -> Self {
        Self(x, y, z)
    }

    #[inline]
    /// system x
    pub fn x(&self) -> SystemUnit {
        self.0
    }

    #[inline]
    /// sets system x
    pub fn set_x(&mut self, x: SystemUnit) {
        self.0 = x;
    }

    #[inline]
    /// system y
    pub fn y(&self) -> SystemUnit {
        self.1
    }

    #[inline]
    /// sets system y
    pub fn set_y(&mut self, y: SystemUnit) {
        self.1 = y;
    }

    #[inline]
    /// system z
    pub fn z(&self) -> SystemUnit {
        self.2
    }

    #[inline]
    /// sets system z
    pub fn set_z(&mut self, z: SystemUnit) {
        self.2 = z;
    }

    /// Computes the absolute value of every coordinate
    pub fn abs(&self) -> Self {
        Self(self.0.abs(), self.1.abs(), self.2.abs())
    }

    /// Computes the maximum element
    pub fn max_element(&self) -> SystemUnit {
        self.0.max(self.1).max(self.2)
    }
}

impl Add<UniverseSystem> for UniverseSystem {
    type Output = UniverseSystem;

    fn add(self, rhs: UniverseSystem) -> Self::Output {
        UniverseSystem(self.0 + rhs.0, self.1 + rhs.1, self.2 + rhs.2)
    }
}

impl Display for Sector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("{}, {}, {}", self.0, self.1, self.2).as_str())?;

        Ok(())
    }
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({:.3}, {:.3}, {:.3}), [{}]",
            self.local.x, self.local.y, self.local.z, self.sector
        )
    }
}

impl Add<Vec3> for Location {
    type Output = Location;

    fn add(self, rhs: Vec3) -> Self::Output {
        let mut loc = Location::new(self.local + rhs, self.sector);
        loc.fix_bounds();
        loc
    }
}

impl Sub<Vec3> for Location {
    type Output = Location;

    fn sub(self, rhs: Vec3) -> Self::Output {
        let mut loc = Location::new(self.local - rhs, self.sector);
        loc.fix_bounds();
        loc
    }
}

impl Sub<Sector> for Sector {
    type Output = Sector;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0, self.1 - rhs.1, self.2 - rhs.2)
    }
}

impl Sub<Location> for Location {
    type Output = Location;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut loc = Location::new(self.local - rhs.local, self.sector - rhs.sector);
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
            val.sector.x() as f32 * SECTOR_DIMENSIONS + val.local.x,
            val.sector.y() as f32 * SECTOR_DIMENSIONS + val.local.y,
            val.sector.z() as f32 * SECTOR_DIMENSIONS + val.local.z,
        )
    }
}

impl Location {
    /// Creates a new location at these coordinates
    pub fn new(local: Vec3, sector: Sector) -> Self {
        Self {
            local,
            sector,
            last_transform_loc: Some(local),
        }
    }

    /// Gets the system coordinates this location is in
    pub fn get_system_coordinates(&self) -> UniverseSystem {
        UniverseSystem(
            (self.sector.x() / SYSTEM_SECTORS as SectorUnit) as SystemUnit,
            (self.sector.y() / SYSTEM_SECTORS as SectorUnit) as SystemUnit,
            (self.sector.z() / SYSTEM_SECTORS as SectorUnit) as SystemUnit,
        )
    }

    /// Gets the sector coordinates as a tuple
    #[inline]
    pub fn sector(&self) -> Sector {
        self.sector
    }

    /// Ensures `self.local` is within [`-SECTOR_DIMENSIONS/2.0`, `SECTOR_DIMENSIONS/2.0`]
    ///
    /// If not, the sector coordinates & `local` will be modified to maintain this
    pub fn fix_bounds(&mut self) {
        let over_x = (self.local.x / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_x != 0 {
            self.local.x -= over_x as f32 * SECTOR_DIMENSIONS;
            self.sector.set_x(self.sector.x() + over_x);
        }

        let over_y = (self.local.y / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_y != 0 {
            self.local.y -= over_y as f32 * SECTOR_DIMENSIONS;
            self.sector.set_y(self.sector.y() + over_y);
        }

        let over_z = (self.local.z / (SECTOR_DIMENSIONS / 2.0)) as i64;
        if over_z != 0 {
            self.local.z -= over_z as f32 * SECTOR_DIMENSIONS;
            self.sector.set_z(self.sector.z() + over_z);
        }
    }

    /// Only usable over f32 distances - will return infinity for distances that are outside the bounds of f32 calculations
    pub fn relative_coords_to(&self, other: &Location) -> Vec3 {
        let (dsx, dsy, dsz) = (
            (other.sector.x() - self.sector.x()) as f32,
            (other.sector.y() - self.sector.y()) as f32,
            (other.sector.z() - self.sector.z()) as f32,
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
        self.sector = other.sector;
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

        let local_x = BigDecimal::from_f32(self.local.x)
            .unwrap_or_else(|| panic!("Died on {}", self.local.x));
        let local_y = BigDecimal::from_f32(self.local.y)
            .unwrap_or_else(|| panic!("Died on {}", self.local.y));
        let local_z = BigDecimal::from_f32(self.local.z)
            .unwrap_or_else(|| panic!("Died on {}", self.local.z));

        Vector3::new(
            BigDecimal::from_i64(self.sector.x()).unwrap() * &sector_dims + local_x,
            BigDecimal::from_i64(self.sector.y()).unwrap() * &sector_dims + local_y,
            BigDecimal::from_i64(self.sector.z()).unwrap() * &sector_dims + local_z,
        )
    }

    /// Returns the coordinates of this location based off 0, 0, 0.
    ///
    /// Useful for short/medium-distance calculations/displaying
    pub fn absolute_coords_f32(&self) -> Vec3 {
        Vec3::new(
            self.sector.x() as f32 * SECTOR_DIMENSIONS + self.local.x,
            self.sector.y() as f32 * SECTOR_DIMENSIONS + self.local.y,
            self.sector.z() as f32 * SECTOR_DIMENSIONS + self.local.z,
        )
    }

    /// Returns the coordinates of this location based off 0, 0, 0.
    ///
    /// Useful for short/medium-distance/semi-large calculations/displaying
    pub fn absolute_coords_f64(&self) -> Vector3<f64> {
        Vector3::new(
            self.sector.x() as f64 * SECTOR_DIMENSIONS as f64 + self.local.x as f64,
            self.sector.y() as f64 * SECTOR_DIMENSIONS as f64 + self.local.y as f64,
            self.sector.z() as f64 * SECTOR_DIMENSIONS as f64 + self.local.z as f64,
        )
    }
}

#[derive(Component, Debug, Reflect, FromReflect, Deref, DerefMut, Clone, Copy)]
/// Stores the location from the previous frame
pub struct PreviousLocation(pub Location);

/// Recursively goes from the top of the parent tree to the bottom and lines up all their locations.
///
/// This probably works.
fn sync_self_with_parents(
    this_entity: Entity,
    parent_query: &Query<&Parent>,
    data_query: &mut Query<(
        &mut Location,
        &mut Transform,
        &mut PreviousLocation,
        &GlobalTransform,
    )>,
) {
    if let Ok(parent) = parent_query.get(this_entity).map(|p| p.get()) {
        sync_self_with_parents(parent, parent_query, data_query);

        let Ok((parent_loc, parent_global_trans)) = data_query.get(parent).map(|(loc, _, _, parent_global_trans)| (*loc, parent_global_trans.translation())) else {
            return;
        };

        let Ok((mut my_loc, mut my_transform, mut my_prev_loc, my_global_trans)) = data_query.get_mut(this_entity) else {
            return;
        };

        if my_loc.last_transform_loc.is_some() {
            let mut my_delta_loc = (*my_loc - my_prev_loc.0).absolute_coords_f32();

            println!(
                "My loc: {}; Last Loc: {}; Delta: {my_delta_loc}",
                *my_loc, my_prev_loc.0
            );

            #[cfg(feature = "server")]
            {
                my_delta_loc = Vec3::ZERO;
            }

            my_transform.translation += my_delta_loc;

            let delta_from_parent = my_global_trans.translation() - parent_global_trans;

            // println!("{parent_loc} + {delta_from_parent} + {my_delta_loc}");

            let my_new_loc = parent_loc + delta_from_parent + my_delta_loc;

            println!("My new loc should be: {my_new_loc}");
            my_loc.set_from(&my_new_loc);
            my_loc.last_transform_loc = Some(my_transform.translation);
            my_prev_loc.0 = *my_loc;
        }
    }
}

/// Adds the previous location component. Put this before the sync bodies & transform
pub fn add_previous_location(
    mut query: Query<(Entity, &Location, Option<&mut PreviousLocation>)>,
    mut commands: Commands,
) {
    for (entity, loc, prev_loc) in query.iter_mut() {
        if let Some(mut prev_loc) = prev_loc {
            prev_loc.0 = *loc;
        } else {
            commands.entity(entity).insert(PreviousLocation(*loc));
        }
    }
}

/// Handles children and their locations.
pub fn handle_child_syncing(
    initial_query: Query<Entity, (Without<Children>, Without<ChunkEntity>)>,
    parent_query: Query<&Parent>,
    mut data_query: Query<(
        &mut Location,
        &mut Transform,
        &mut PreviousLocation,
        &GlobalTransform,
    )>,
) {
    for entity in initial_query.iter() {
        sync_self_with_parents(entity, &parent_query, &mut data_query);
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Location>()
        .register_type::<PreviousLocation>();
}

#[cfg(test)]
mod tests {
    use bevy::prelude::Vec3;

    use crate::physics::location::{Sector, SECTOR_DIMENSIONS};

    use super::Location;

    #[test]
    fn in_same_sector_pos() {
        let l1 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));

        let result = Vec3::new(30.0, 30.0, 30.0);

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_same_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(20, -20, 20));

        let result = Vec3::new(-30.0, -30.0, -30.0);

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_diff_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(19, -21, 19));

        let result = Vec3::new(
            -30.0 - SECTOR_DIMENSIONS,
            -30.0 - SECTOR_DIMENSIONS,
            -30.0 - SECTOR_DIMENSIONS,
        );

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_diff_sector_pos() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(21, -19, 21));

        let result = Vec3::new(
            -30.0 + SECTOR_DIMENSIONS,
            -30.0 + SECTOR_DIMENSIONS,
            -30.0 + SECTOR_DIMENSIONS,
        );

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_far_sector_pos() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(30, -10, 30));

        let result = Vec3::new(
            -30.0 + SECTOR_DIMENSIONS * 10.0,
            -30.0 + SECTOR_DIMENSIONS * 10.0,
            -30.0 + SECTOR_DIMENSIONS * 10.0,
        );

        assert_eq!(l1.relative_coords_to(&l2), result);
    }

    #[test]
    fn in_far_sector_neg() {
        let l1 = Location::new(Vec3::new(15.0, 15.0, 15.0), Sector::new(20, -20, 20));
        let l2 = Location::new(Vec3::new(-15.0, -15.0, -15.0), Sector::new(10, -30, 10));

        let result = Vec3::new(
            -30.0 - SECTOR_DIMENSIONS * 10.0,
            -30.0 - SECTOR_DIMENSIONS * 10.0,
            -30.0 - SECTOR_DIMENSIONS * 10.0,
        );

        assert_eq!(l1.relative_coords_to(&l2), result);
    }
}
