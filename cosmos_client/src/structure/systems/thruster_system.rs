use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    structure::ship::ship_movement::ShipMovement,
};

use crate::state::game_state::GameState;

fn apply_thruster_sound(
    query: Query<(Entity, &ShipMovement, Option<&AudioEmitter>)>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handle: Res<ThrusterAudioHandle>,
) {
    for (entity, ship_movement, audio_emitter) in query.iter() {
        // `ship_movement.movement == Vec3::ZERO` is a hacky way to determine if the thrusters are off, come up with a better solution later.
        let thrusters_off = false; // ship_movement.movement == Vec3::ZERO && ship_movement.torque == Vec3::ZERO;

        // println!("{ship_movement}");

        if thrusters_off && audio_emitter.is_some() {
            commands.entity(entity).remove::<AudioEmitter>();
            println!("Removing emitter!");
        } else if !thrusters_off && audio_emitter.is_none() {
            let playing_sound = audio.play(audio_handle.0.clone()).looped().with_volume(0.1).handle();

            commands.entity(entity).insert(AudioEmitter {
                instances: vec![playing_sound],
            });

            println!("Adding emitter!");
        }
    }
}

#[derive(Resource)]
struct LoadingAudioHandle(Handle<AudioSource>, usize);

#[derive(Resource)]
struct ThrusterAudioHandle(Handle<AudioSource>);

fn prepare(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut loader: ResMut<LoadingManager>,
    mut event_writer: EventWriter<AddLoadingEvent>,
) {
    let id = loader.register_loader(&mut event_writer);

    commands.insert_resource(LoadingAudioHandle(asset_server.load("cosmos/sounds/sfx/thruster-running.ogg"), id));

    println!("Started loading sound!");
}

fn check(
    handle: Option<Res<LoadingAudioHandle>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut loader: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
) {
    if let Some(handle) = handle {
        if asset_server.get_load_state(handle.0.id()) == LoadState::Loaded {
            commands.insert_resource(ThrusterAudioHandle(handle.0.clone()));
            commands.remove_resource::<LoadingAudioHandle>();

            println!("Done loading sound!");

            loader.finish_loading(handle.1, &mut end_writer);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PreLoading), prepare)
        .add_systems(Update, check.run_if(in_state(GameState::PreLoading)))
        .add_systems(Update, apply_thruster_sound.run_if(in_state(GameState::Playing)))
        .insert_resource(SpacialAudio { max_distance: 100.0 }); // TODO: Move this to a seperate place
}
