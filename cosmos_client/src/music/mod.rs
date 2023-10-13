use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::*;
use std::time::Duration;

#[derive(Resource)]
struct BackgroundSong(Handle<AudioInstance>);

// `InspectorOptions` are completely optional
#[derive(Reflect, Resource, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct VolumeSetting(#[inspector(min = 0.0, max = 1.0)] f64);

impl VolumeSetting {
    /// Returns the volume as a decimal percent [0.0, 1.0]
    pub fn percent(&self) -> f64 {
        self.0
    }
}

impl Default for VolumeSetting {
    fn default() -> Self {
        Self(1.0)
    }
}

fn monitor_background_song(bg_song: Res<BackgroundSong>, mut audio_instances: ResMut<Assets<AudioInstance>>, mut commands: Commands) {
    if let Some(instance) = audio_instances.get_mut(&bg_song.0) {
        match instance.state() {
            PlaybackState::Stopped => {
                commands.remove_resource::<BackgroundSong>();
            }
            _ => {}
        }
    }
}

fn start_playing(mut commands: Commands, asset_server: Res<AssetServer>, audio: Res<Audio>, volume: Res<VolumeSetting>) {
    let handle = audio
        .play(asset_server.load("cosmos/sounds/music/AntirockSong.ogg"))
        .with_volume(volume.percent())
        .fade_in(AudioTween::new(Duration::from_secs(2), AudioEasing::InOutPowi(2)))
        .handle();

    commands.insert_resource(BackgroundSong(handle));
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(AudioPlugin)
        .add_systems(
            Update,
            (
                start_playing.run_if(not(resource_exists::<BackgroundSong>())),
                monitor_background_song.run_if(resource_exists::<BackgroundSong>()),
            ),
        )
        .init_resource::<VolumeSetting>();
}
