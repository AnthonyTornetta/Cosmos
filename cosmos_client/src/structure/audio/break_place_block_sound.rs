use bevy::prelude::{
    resource_exists, App, BuildChildren, ChildBuild, Commands, EventReader, Handle, IntoSystemConfigs, Name, Query, Res, Resource,
    Transform, Update,
};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    block::block_events::BlockEventsSet,
    netty::system_sets::NetworkingSystemsSet,
    state::GameState,
    structure::{shared::DespawnWithStructure, Structure},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    events::block::block_events::{RequestBlockBreakEvent, RequestBlockPlaceEvent},
};

fn play_block_break_sound(
    mut event_reader: EventReader<RequestBlockBreakEvent>,
    break_sound: Res<BlockBreakSound>,
    structure_query: Query<&Structure>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let Ok(structure) = structure_query.get(ev.block.structure()) else {
            continue;
        };

        let sound_location = structure.block_relative_position(ev.block.coords());

        let playing_sound: Handle<AudioInstance> = audio.play(break_sound.0.clone()).with_volume(0.0).handle();

        let sound_emission = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 16.0,
            ..Default::default()
        }]);

        commands.entity(ev.block.structure()).with_children(|p| {
            p.spawn((
                Name::new("Block break sound"),
                DespawnWithStructure,
                DespawnOnNoEmissions,
                Transform::from_translation(sound_location),
                sound_emission,
            ));
        });
    }
}

fn play_block_place_sound(
    mut event_reader: EventReader<RequestBlockPlaceEvent>,
    place_sound: Res<BlockPlaceSound>,
    structure_query: Query<&Structure>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    for ev in event_reader.read() {
        let Ok(structure) = structure_query.get(ev.block.structure()) else {
            continue;
        };

        let sound_location = structure.block_relative_position(ev.block.coords());

        let playing_sound: Handle<AudioInstance> = audio.play(place_sound.0.clone()).with_volume(0.0).handle();

        let sound_emission = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 16.0,
            ..Default::default()
        }]);

        commands.entity(ev.block.structure()).with_children(|p| {
            p.spawn((
                Name::new("Block break sound"),
                DespawnWithStructure,
                DespawnOnNoEmissions,
                Transform::from_translation(sound_location),
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
    load_assets::<AudioSource, LoadingBreakAudio, 2>(
        app,
        GameState::Loading,
        ["cosmos/sounds/sfx/break.ogg", "cosmos/sounds/sfx/place.ogg"],
        |mut commands, [s_break, s_place]| {
            commands.insert_resource(BlockBreakSound(s_break.0));
            commands.insert_resource(BlockPlaceSound(s_place.0));
        },
    );

    app.add_systems(
        Update,
        (
            play_block_place_sound.run_if(resource_exists::<BlockPlaceSound>),
            play_block_break_sound.run_if(resource_exists::<BlockBreakSound>),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .after(BlockEventsSet::SendEventsForNextFrame),
    );
}
