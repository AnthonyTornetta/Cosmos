use bevy::prelude::{
    resource_exists, App, BuildChildren, ChildBuild, Commands, EventReader, Handle, IntoSystemConfigs, Name, Query, Res, Resource,
    Transform, Update,
};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    block::block_events::BlockEventsSet,
    netty::system_sets::NetworkingSystemsSet,
    state::GameState,
    structure::{block_health::events::BlockTakeDamageEvent, shared::DespawnWithStructure, Structure},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
};

fn play_block_damage_sound(
    mut event_reader: EventReader<BlockTakeDamageEvent>,
    engine_idle_sound: Res<BlockDamageSound>,
    structure_query: Query<&Structure>,
    audio: Res<Audio>,
    mut commands: Commands,
) {
    //    let mut closest = None;
    for ev in event_reader.read() {
        let Ok(structure) = structure_query.get(ev.structure_entity) else {
            continue;
        };

        let sound_location = structure.block_relative_position(ev.block.coords());

        let playing_sound: Handle<AudioInstance> = audio.play(engine_idle_sound.0.clone()).with_volume(0.0).handle();

        let sound_emission = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
            instance: playing_sound,
            max_distance: 100.0,
            ..Default::default()
        }]);

        commands.entity(ev.structure_entity).with_children(|p| {
            p.spawn((
                Name::new("Block take damage sound"),
                DespawnWithStructure,
                DespawnOnNoEmissions,
                Transform::from_translation(sound_location),
                sound_emission,
            ));
        });
    }
}

#[derive(Resource)]
struct BlockDamageSound(Handle<AudioSource>);

struct LoadingBlockAudio;

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, LoadingBlockAudio, 1>(app, GameState::Loading, ["cosmos/sounds/sfx/thud.ogg"], |mut commands, [sound]| {
        commands.insert_resource(BlockDamageSound(sound.0));
    });

    app.add_systems(
        Update,
        play_block_damage_sound
            .in_set(BlockEventsSet::ProcessEvents)
            .in_set(NetworkingSystemsSet::Between)
            .run_if(resource_exists::<BlockDamageSound>),
    );
}
