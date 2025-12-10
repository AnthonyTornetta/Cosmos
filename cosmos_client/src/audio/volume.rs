//! Volume stuff

use std::ops::Mul;

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::Decibels;
use cosmos_core::registry::Registry;

use crate::settings::{Setting, SettingsRegistry, SettingsSet};

#[derive(Resource, Debug, Reflect, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
/// Messageually this will be present for every sound type in the game, but for now this only is for music
pub struct MasterVolume(Volume);

impl MasterVolume {
    /// Multiply the volume of something by this to apply this volume's effect.
    pub fn get(&self) -> Volume {
        self.0
    }
}

/// Volume as a percentage (0.0 = silent, 1.0 = loudest)
#[derive(Debug, Reflect, InspectorOptions, Clone, Copy)]
pub struct Volume(#[inspector(min = 0.0, max = 1.0)] f32);

impl From<Volume> for Decibels {
    fn from(value: Volume) -> Self {
        value.as_decibels()
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Volume {
    /// The volume at which no sound is produced
    pub const MIN: Volume = Volume(0.0);

    /// Creates a new volume and ensures the value is within 0.0 to 1.0
    ///
    /// See [`Self::new_unbound`] if you want extra loud volumes
    pub fn new(volume: f32) -> Self {
        Self(volume.clamp(0.0, 1.0))
    }

    /// Creates a new volume type without automatically ensuring volume is within 0.0 to 1.0 range.
    ///
    /// This can be used to make extra impactful sounds if you really want
    pub fn new_unbound(volume: f32) -> Self {
        Self(volume)
    }

    /// Computes this volume in decibel form (0.0 = silent, 1.0 = normal volume)
    pub fn as_decibels(&self) -> Decibels {
        ((self.0.powf(0.3) * -Decibels::SILENCE.0) + Decibels::SILENCE.0).into()
    }

    /// Returns this as a percentage float [0.0 - 1.0] for normal ranges, but can exceed these
    /// bounds if initialized with [`Self::new_unbound`].
    pub fn as_percent(&self) -> f32 {
        self.0
    }
}

impl Mul<Volume> for Volume {
    type Output = Self;

    fn mul(self, rhs: Volume) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

fn load_volume(settings: Res<Registry<Setting>>, mut music_volume: ResMut<MasterVolume>) {
    music_volume.0 = Volume::new(settings.i32_or("cosmos:master_volume", 100) as f32 / 100.0);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (load_volume).in_set(SettingsSet::LoadSettings))
        .init_resource::<MasterVolume>()
        .register_type::<MasterVolume>();
}
