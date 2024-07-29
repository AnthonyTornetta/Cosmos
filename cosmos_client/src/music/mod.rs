//! Handles the background music of cosmos

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    netty::client::LocalPlayer,
    physics::location::{Location, LocationPhysicsSet, Sector},
};
use std::time::Duration;

#[derive(Resource)]
struct BackgroundSong(Handle<AudioInstance>);

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

fn monitor_background_song(bg_song: Res<BackgroundSong>, mut audio_instances: ResMut<Assets<AudioInstance>>, mut commands: Commands) {
    if let Some(instance) = audio_instances.get_mut(&bg_song.0) {
        if instance.state() == PlaybackState::Stopped {
            commands.remove_resource::<BackgroundSong>();
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

fn adjust_volume(mut audio_instances: ResMut<Assets<AudioInstance>>, volume: Res<VolumeSetting>, background_song: Res<BackgroundSong>) {
    let Some(instance) = audio_instances.get_mut(&background_song.0) else {
        return;
    };

    instance.set_volume(volume.0, AudioTween::default());
}

#[derive(Event, Default)]
/// Signals that the music system should play a background song
struct PlayMusicEvent;

#[derive(Component)]
struct LastPlaySector(Sector);

fn play_music_when_enter_new_sector(
    query: Query<(Entity, &Location, Option<&LastPlaySector>), With<LocalPlayer>>,
    mut commands: Commands,
    mut event_writer: EventWriter<PlayMusicEvent>,
    background_song: Option<Res<BackgroundSong>>,
) {
    let Ok((player_entity, location, last_play_sector)) = query.get_single() else {
        return;
    };

    if let Some(last_play_sector) = last_play_sector {
        if background_song.is_none() && last_play_sector.0 != location.sector {
            event_writer.send_default();
        }
    }

    commands.entity(player_entity).insert(LastPlaySector(location.sector));
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(AudioPlugin)
        .add_systems(
            Update,
            (
                start_playing
                    .run_if(on_event::<PlayMusicEvent>())
                    .run_if(not(resource_exists::<BackgroundSong>)),
                monitor_background_song.run_if(resource_exists::<BackgroundSong>),
                adjust_volume
                    .run_if(resource_changed::<VolumeSetting>)
                    .run_if(resource_exists::<BackgroundSong>),
                play_music_when_enter_new_sector.after(LocationPhysicsSet::DoPhysics),
            )
                .chain(),
        )
        .init_resource::<VolumeSetting>()
        .register_type::<VolumeSetting>()
        .add_event::<PlayMusicEvent>();
}
