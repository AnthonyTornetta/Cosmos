//! Responsible for building ships for the client.

use bevy::prelude::*;
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    state::GameState,
    structure::{loading::StructureLoadingSet, shared::DespawnWithStructure, ship::Ship},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, volume::Volume},
};

fn client_on_add_ship(
    query: Query<Entity, Added<Ship>>,
    engine_idle_sound: Res<EngineIdleSound>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        let playing_sound: Handle<AudioInstance> = audio.play(engine_idle_sound.0.clone()).with_volume(Volume::MIN).looped().handle();

        let idle_emitter = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 20.0,
            peak_volume: Volume::new(0.3), // magic number that sounds good
            ..Default::default()
        }]);

        commands.entity(entity).with_children(|p| {
            p.spawn((
                Name::new("Engine idle sound"),
                DespawnWithStructure,
                Transform::from_xyz(0.5, 0.5, 0.5),
                idle_emitter,
            ));
        });
    }
}

#[derive(Resource)]
struct EngineIdleSound(Handle<AudioSource>);

struct LoadingShipAudio;

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, LoadingShipAudio, 1>(
        app,
        GameState::PreLoading,
        ["cosmos/sounds/sfx/engine-idle.ogg"],
        |mut commands, [sound]| {
            commands.insert_resource(EngineIdleSound(sound.0));
        },
    );

    app.add_systems(
        FixedUpdate,
        client_on_add_ship
            .in_set(StructureLoadingSet::AddStructureComponents)
            .run_if(resource_exists::<EngineIdleSound>),
    );
}
