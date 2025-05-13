//! Used to track total game time

use std::{ops::Sub, time::Duration};

use bevy::{prelude::Resource, reflect::Reflect};
use serde::{Deserialize, Serialize};

#[derive(Resource, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Reflect, Default)]
/// How long this universe (game) has been around for
pub struct UniverseTimestamp(u64);

impl Sub<UniverseTimestamp> for UniverseTimestamp {
    type Output = Option<Duration>;

    fn sub(self, rhs: Self) -> Self::Output {
        if rhs.0 > self.0 {
            return None;
        }

        Some(Duration::from_secs(self.0 - rhs.0))
    }
}

impl UniverseTimestamp {
    /// Creates a new timestamp based on this amount of seconds
    pub fn new(secs: u64) -> Self {
        Self(secs)
    }

    /// Returns the number of seconds this timestamp represents
    pub fn as_secs(&self) -> u64 {
        self.0
    }

    /// Returns this timestamp as a duration since the beginning of the world
    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(self.0)
    }

    /// Advances this timestamp by a set number of seconds
    pub fn advance_by(&mut self, secs: u64) {
        self.0 += secs;
    }

    /// Advances this timestamp by one second
    pub fn tick(&mut self) {
        self.advance_by(1)
    }
}
