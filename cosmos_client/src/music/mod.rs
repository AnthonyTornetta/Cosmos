use std::time::Duration;

use bevy::prelude::{not, resource_exists, App, AssetServer, Assets, Commands, Handle, IntoSystemConfigs, Res, ResMut, Resource, Update};
use bevy_kira_audio::{Audio, AudioControl, AudioEasing, AudioInstance, AudioPlugin, AudioTween, PlaybackState};

#[derive(Resource)]
struct BackgroundSong(Handle<AudioInstance>);

pub(super) fn register(app: &mut App) {
    app.add_plugins(AudioPlugin).add_systems(
        Update,
        (
            start_playing.run_if(not(resource_exists::<BackgroundSong>())),
            monitor_background_song.run_if(resource_exists::<BackgroundSong>()),
        ),
    );
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

fn start_playing(mut commands: Commands, asset_server: Res<AssetServer>, audio: Res<Audio>) {
    let handle = audio
        .play(asset_server.load("cosmos/sounds/music/AntirockSong.ogg"))
        .with_volume(0.15)
        .fade_in(AudioTween::new(Duration::from_secs(2), AudioEasing::InOutPowi(2)))
        .handle();

    commands.insert_resource(BackgroundSong(handle));
}
