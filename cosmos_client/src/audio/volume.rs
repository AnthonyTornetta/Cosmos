//! Volume stuff

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use cosmos_core::registry::Registry;

use crate::settings::{Setting, SettingsRegistry, SettingsSet};

#[derive(Resource, Debug, Reflect, Default, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
/// Eventually this will be present for every sound type in the game, but for now this only is for music
pub struct MasterVolume(#[inspector(min = 0.0, max = 1.0)] f64);

impl MasterVolume {
    /// Multiply the volume of something by this to apply this volume's effect.
    pub fn multiplier(&self) -> f64 {
        self.0.powi(2)
    }
}

fn load_volume(settings: Res<Registry<Setting>>, mut music_volume: ResMut<MasterVolume>) {
    music_volume.0 = settings.i32_or("cosmos:master_volume", 100) as f64 / 100.0;
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (load_volume).in_set(SettingsSet::LoadSettings))
        .init_resource::<MasterVolume>()
        .register_type::<MasterVolume>();
}
