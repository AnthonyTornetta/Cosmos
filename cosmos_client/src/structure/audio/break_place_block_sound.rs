use bevy::{
    prelude::{
        resource_exists, App, BuildChildren, Commands, EventReader, Handle, IntoSystemConfigs, Name, Query, Res, Resource, Transform,
        Update,
    },
    transform::TransformBundle,
};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    block::block_events::BlockBreakEvent,
    structure::{ship::core::DespawnWithStructure, Structure},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
};

fn play_block_break_sound(
    mut event_reader: EventReader<BlockBreakEvent>,
    break_sound: Res<BlockBreakSound>,
    structure_query: Query<&Structure>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    for ev in event_reader.iter() {
        let Ok(structure) = structure_query.get(ev.structure_entity) else {
            continue;
        };

        let sound_location = structure.block_relative_position(ev.block.coords());

        let playing_sound: Handle<AudioInstance> = audio.play(break_sound.0.clone()).with_volume(0.0).handle();

        let sound_emission = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 16.0,
            ..Default::default()
        }]);

        commands.entity(ev.structure_entity).with_children(|p| {
            p.spawn((
                Name::new("Block break sound"),
                DespawnWithStructure,
                DespawnOnNoEmissions,
                TransformBundle::from_transform(Transform::from_translation(sound_location)),
                sound_emission,
            ));
        });
    }
}

#[derive(Resource)]
struct BlockBreakSound(Handle<AudioSource>);
#[derive(Resource)]
struct BlockPlaceSound(Handle<AudioSource>);

struct LoadingPlaceAudio;
struct LoadingBreakAudio;

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, LoadingBreakAudio>(
        app,
        GameState::Loading,
        vec!["cosmos/sounds/sfx/break.ogg"],
        |mut commands, mut sounds| {
            let sound = sounds.remove(0);

            commands.insert_resource(BlockBreakSound(sound.0));
        },
    );

    load_assets::<AudioSource, LoadingPlaceAudio>(
        app,
        GameState::Loading,
        vec!["cosmos/sounds/sfx/place.ogg"],
        |mut commands, mut sounds| {
            let sound = sounds.remove(0);

            commands.insert_resource(BlockPlaceSound(sound.0));
        },
    );

    app.add_systems(Update, play_block_break_sound.run_if(resource_exists::<BlockBreakSound>()));
}
