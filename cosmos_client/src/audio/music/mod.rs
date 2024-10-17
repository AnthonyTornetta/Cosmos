//! Handles the background music of cosmos

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::*;
use dynamic_music::MusicAtmosphere;

pub mod dynamic_music;

#[derive(Resource)]
struct PlayingBackgroundSong(Handle<AudioInstance>);

#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
/// Eventually this will be present for every sound type in the game, but for now this only is for music
pub struct VolumeSetting(#[inspector(min = 0.0, max = 1.0)] f64);

impl VolumeSetting {
    /// Returns the volume as a decimal percent [0.0, 1.0]
    pub fn percent(&self) -> f64 {
        self.0
    }
}

impl Default for VolumeSetting {
    /// Initializes the volume to 0.2 (20%).
    fn default() -> Self {
        Self(0.2) // 1.0 is way too loud - in the future introduce a global volume
    }
}

fn monitor_background_song(
    bg_song: Res<PlayingBackgroundSong>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    mut commands: Commands,
) {
    if let Some(instance) = audio_instances.get_mut(&bg_song.0) {
        if instance.state() == PlaybackState::Stopped {
            commands.remove_resource::<PlayingBackgroundSong>();
        }
    }
}

fn adjust_volume(
    mut audio_instances: ResMut<Assets<AudioInstance>>,
    volume: Res<VolumeSetting>,
    background_song: Res<PlayingBackgroundSong>,
) {
    let Some(instance) = audio_instances.get_mut(&background_song.0) else {
        return;
    };

    instance.set_volume(volume.0, AudioTween::default());
}

#[derive(Event)]
/// Signals that the music system should play a background song
pub struct PlayMusicEvent {
    /// The atmosphere of the song that should be played
    pub atmosphere: MusicAtmosphere,
}

pub(super) fn register(app: &mut App) {
    dynamic_music::register(app);

    app.add_plugins(AudioPlugin)
        .add_systems(
            Update,
            (
                monitor_background_song.run_if(resource_exists::<PlayingBackgroundSong>),
                adjust_volume
                    .run_if(resource_changed::<VolumeSetting>)
                    .run_if(resource_exists::<PlayingBackgroundSong>),
            )
                .chain(),
        )
        .init_resource::<VolumeSetting>()
        .register_type::<VolumeSetting>()
        .add_event::<PlayMusicEvent>();
}
