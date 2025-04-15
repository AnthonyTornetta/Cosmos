//! Controls the playing of music as the player explores the universe

use std::{fs, time::Duration};

use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioEasing, AudioSource, AudioTween};
use cosmos_core::{state::GameState, utils::random::random_range};
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};

use crate::audio::volume::MasterVolume;

use super::{MusicVolume, PlayMusicEvent, PlayingBackgroundSong};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Reflect)]
/// Describes the "Atmosphere"/mood the music should be played in.
pub enum MusicAtmosphere {
    /// A calm environment, such as peacefully flying around or walking on a planet
    Calm,
    /// Intense action, such as being in combat
    Intense,
}

#[derive(Clone, PartialEq, Eq, Debug, Reflect)]
/// A song that plays in the background.
///
/// These are automatically generated by looking in the `assets/cosmos/sounds/music` directory.
/// Make sure each `.ogg` file has a `.json` file with the same name that specifies its metadata.
pub struct BackgroundSong {
    atmosphere: MusicAtmosphere,
    handle: Handle<AudioSource>,
}

#[derive(Default, Reflect, Resource, InspectorOptions, Debug)]
#[reflect(Resource, InspectorOptions)]
/// Contains the music the game can play
pub struct MusicController {
    songs: Vec<BackgroundSong>,
}

impl MusicController {
    /// Adds a song that can be played
    pub fn add_song(&mut self, song: BackgroundSong) {
        self.songs.push(song);
    }

    /// Selects a random song that matches this atmosphere
    pub fn random_song(&self, atmosphere: MusicAtmosphere) -> Option<&BackgroundSong> {
        self.songs
            .iter()
            .filter(|x| x.atmosphere == atmosphere)
            .choose(&mut rand::rng())
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct MusicDefinition {
    atmosphere: MusicAtmosphere,
}

fn load_default_songs(asset_server: Res<AssetServer>, mut music_controller: ResMut<MusicController>) {
    let Ok(entries) = fs::read_dir("assets/cosmos/sounds/music/") else {
        error!("Missing music directory - unable to load background music!");
        return;
    };

    for entry in entries {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                let Some(extension) = path.extension() else {
                    continue;
                };
                if !extension.eq_ignore_ascii_case("ogg") {
                    continue;
                }

                let def_path = path.with_extension("json");
                let Ok(info_file) = fs::read_to_string(&def_path) else {
                    error!("Missing music info file for {path:?}");
                    continue;
                };

                let music_def = match serde_json::from_str::<MusicDefinition>(&info_file) {
                    Ok(music_def) => music_def,
                    Err(e) => {
                        error!("Invalid music definition file for {:?}\n{e:?}", def_path);
                        continue;
                    }
                };

                let song = BackgroundSong {
                    atmosphere: music_def.atmosphere,
                    handle: asset_server.load(format!("cosmos/sounds/music/{}", path.file_name().expect("How?").to_str().unwrap())),
                };

                info!("Adding song {song:?}");

                music_controller.add_song(song);
            }
            Err(e) => {
                error!("{e:?}");
            }
        }
    }
}

fn start_playing(
    mut commands: Commands,
    mut evr_play_music: EventReader<PlayMusicEvent>,
    audio: Res<Audio>,
    volume: Res<MusicVolume>,
    master_volume: Res<MasterVolume>,
    jukebox: Res<MusicController>,
) {
    let Some(ev) = evr_play_music.read().next() else {
        return;
    };

    let Some(song) = jukebox.random_song(ev.atmosphere) else {
        warn!("Missing song for atmosphere: {:?}", ev.atmosphere);
        return;
    };

    let handle = audio
        .play(song.handle.clone())
        .with_volume(volume.percent() * master_volume.multiplier())
        .fade_in(AudioTween::new(Duration::from_secs(2), AudioEasing::InOutPowi(2)))
        .handle();

    commands.insert_resource(PlayingBackgroundSong(handle));
}

#[derive(Reflect, Resource, InspectorOptions, Debug)]
#[reflect(Resource, InspectorOptions)]
struct NextSongTime(f32);
const MIN_DELAY_SEC: f32 = 5.0 * 60.0; // 5min
const MAX_DELAY_SEC: f32 = 20.0 * 60.0; // 20min

fn trigger_music_playing(mut next_song_time: ResMut<NextSongTime>, mut event_writer: EventWriter<PlayMusicEvent>, time: Res<Time>) {
    if next_song_time.0 > time.elapsed_secs() {
        return;
    }

    next_song_time.0 = time.elapsed_secs() + random_range(MIN_DELAY_SEC, MAX_DELAY_SEC);

    event_writer.send(PlayMusicEvent {
        atmosphere: MusicAtmosphere::Calm,
    });
}

pub(super) fn register(app: &mut App) {
    let initial_delay = random_range(MIN_DELAY_SEC, MAX_DELAY_SEC);
    app.init_resource::<MusicController>().insert_resource(NextSongTime(initial_delay));

    app.add_systems(OnEnter(GameState::Loading), load_default_songs);

    app.add_systems(
        Update,
        (trigger_music_playing, start_playing.run_if(on_event::<PlayMusicEvent>))
            .chain()
            .run_if(in_state(GameState::Playing))
            .run_if(not(resource_exists::<PlayingBackgroundSong>)),
    )
    .register_type::<MusicController>()
    .register_type::<NextSongTime>();
}
