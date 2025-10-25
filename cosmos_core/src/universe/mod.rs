//! Contains shared logic for the parts of the universe

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod map;
pub mod star;
pub mod warp;

#[derive(Debug, Serialize, Deserialize, Default, PartialOrd, PartialEq, Clone, Copy)]
/// The danger level in this faction
pub struct SectorDanger {
    // We treat this as a float for future expansions to this logic, but internally store as i8 to save memory.
    danger: i8,
}

/// A way of guaging relative danger
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SectorDangerRange {
    /// The most peaceful type of sector - no pirate activity and frequent NPC encounters.
    VeryPeaceful,
    /// A neutral sector - anything can happen
    Peaceful,
    /// A sector with little to no pirate activity, will frequently encounter friendly NPCs
    Neutral,
    /// A sector capable of spawning random pirate fleets
    Dangerous,
    /// The most dangerous type of sector - typically home to pirate stations
    VeryDangerous,
}

impl SectorDanger {
    /// The midpoint between minimum danger and maximum danger. The neutral point.
    pub const MIDDLE: Self = Self { danger: 0 };

    /// Creates a new danger value bounded between [`SectorDanger::MIN_DANGER`] and
    /// [`SectorDanger::MAX_DANGER`]
    pub const fn new(danger: f32) -> Self {
        Self {
            // .round() doesn't work in const functions, so the +0.5 does it instead
            danger: (danger.clamp(Self::MIN_DANGER, Self::MAX_DANGER) + 0.5) as i8,
        }
    }

    /// Returns the danger as a f32 bounded between [-1.0, 1.0] (negative least danger, positive
    /// most danger)
    pub fn bounded(&self) -> f32 {
        self.danger as f32 / 100.0
    }

    /// Returns the sector danger as a more easy-to-use [`SectorDangerRange`].
    pub fn sector_danger_range(&self) -> SectorDangerRange {
        if self.danger == Self::MIDDLE.danger {
            SectorDangerRange::Neutral
        } else if self.danger > 50 {
            SectorDangerRange::VeryDangerous
        } else if self.danger > 0 {
            SectorDangerRange::Dangerous
        } else if self.danger > -50 {
            SectorDangerRange::Peaceful
        } else {
            SectorDangerRange::VeryPeaceful
        }
    }
}

impl SectorDanger {
    /// The maximum danger value a sector can be
    pub const MAX_DANGER: f32 = 100.0;
    /// The minimum danger value (most peaceful)
    pub const MIN_DANGER: f32 = -100.0;
}

pub(super) fn register(app: &mut App) {
    star::register(app);
    map::register(app);
    warp::register(app);
}
