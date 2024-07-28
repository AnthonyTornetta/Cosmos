//! Client-side laser cannon system logic

use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{
    netty::system_sets::NetworkingSystemsSet,
    physics::location::{Location, LocationPhysicsSet},
    structure::systems::laser_cannon_system::LaserCannonSystem,
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
};

use super::sync::sync_system;

#[derive(Event)]
/// This event is fired whenever a laser cannon system is fired
pub struct LaserCannonSystemFiredEvent(pub Entity);

#[derive(Resource)]
struct LaserCannonFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

fn apply_shooting_sound(
    query: Query<(&Location, &GlobalTransform)>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handles: Res<LaserCannonFireHandles>,
    mut event_reader: EventReader<LaserCannonSystemFiredEvent>,
) {
    for entity in event_reader.read() {
        let Ok((ship_location, ship_global_transform)) = query.get(entity.0) else {
            continue;
        };

        let mut location = *ship_location;
        let translation = ship_global_transform.translation();
        location.last_transform_loc = Some(ship_global_transform.translation());

        let idx = rand::random::<usize>() % audio_handles.0.len();

        let handle = audio_handles.0[idx].clone_weak();

        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone_weak()).handle();

        commands.spawn((
            CosmosAudioEmitter {
                emissions: vec![AudioEmission {
                    instance: playing_sound,
                    handle,
                    ..Default::default()
                }],
            },
            DespawnOnNoEmissions,
            location,
            TransformBundle::from_transform(Transform::from_translation(translation)),
        ));
    }
}

struct LaserCannonLoadingFlag;

pub(super) fn register(app: &mut App) {
    sync_system::<LaserCannonSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, LaserCannonLoadingFlag>(
        app,
        GameState::PreLoading,
        vec![
            "cosmos/sounds/sfx/laser-fire-1.ogg",
            "cosmos/sounds/sfx/laser-fire-2.ogg",
            "cosmos/sounds/sfx/laser-fire-3.ogg",
        ],
        |mut commands, handles| {
            commands.insert_resource(LaserCannonFireHandles(
                handles.into_iter().filter(|x| x.1 == LoadState::Loaded).map(|x| x.0).collect(),
            ));
        },
    );

    app.add_event::<LaserCannonSystemFiredEvent>().add_systems(
        Update,
        apply_shooting_sound
            .after(LocationPhysicsSet::DoPhysics)
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
