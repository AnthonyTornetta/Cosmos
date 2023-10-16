use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager},
    structure::ship::ship_movement::ShipMovement,
};

use crate::{
    audio::{AudioEmission, CosmosAudioEmitter},
    state::game_state::GameState,
};

fn apply_thruster_sound(
    query: Query<(Entity, &ShipMovement, Option<&CosmosAudioEmitter>)>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handle: Res<ThrusterAudioHandle>,
) {
    for (entity, ship_movement, audio_emitter) in query.iter() {
        // A hacky way of determining if the thrusters are running
        let thrusters_off = ship_movement.movement.length_squared() + ship_movement.torque.length_squared() < 0.1;

        if thrusters_off && audio_emitter.is_some() {
            commands.entity(entity).remove::<CosmosAudioEmitter>();
            println!("Removing emitter!");
        } else if !thrusters_off && audio_emitter.is_none() {
            let playing_sound: Handle<AudioInstance> = audio.play(audio_handle.0.clone()).looped().with_volume(0.1).handle();

            commands.entity(entity).insert(CosmosAudioEmitter {
                emissions: vec![AudioEmission {
                    instance: playing_sound,
                    max_distance: 100.0,
                    peak_volume: 0.3,
                    ..Default::default()
                }],
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
        .add_systems(Update, apply_thruster_sound.run_if(in_state(GameState::Playing)));
}
