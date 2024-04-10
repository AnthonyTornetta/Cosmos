use bevy::{
    asset::{Handle, LoadState},
    ecs::{
        query::Added,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Res, Resource},
    },
    prelude::{App, Commands, Entity, Query, Update},
    transform::{
        components::{GlobalTransform, Transform},
        TransformBundle,
    },
};

use bevy_kira_audio::{Audio, AudioControl, AudioInstance, AudioSource};
use cosmos_core::{
    ecs::NeedsDespawned,
    projectiles::missile::{Explosion, ExplosionSystemSet},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, AudioEmitterBundle, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
};

fn respond_to_explosion(
    mut commands: Commands,
    q_explosions: Query<(Entity, &GlobalTransform), Added<Explosion>>,
    audio: Res<Audio>,
    audio_sources: Res<ExplosionAudio>,
) {
    for (ent, g_trans) in q_explosions.iter() {
        commands.entity(ent).insert(NeedsDespawned);

        println!("TODO: Play cool explosion effect instead of despawning");

        let handle = audio_sources.0[rand::random::<usize>() % audio_sources.0.len()].clone_weak();

        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone()).with_volume(0.0).handle();

        commands.spawn((
            DespawnOnNoEmissions,
            AudioEmitterBundle {
                emitter: CosmosAudioEmitter::with_emissions(vec![AudioEmission {
                    instance: playing_sound,
                    handle,
                    max_distance: 200.0,
                    peak_volume: 1.0,
                    ..Default::default()
                }]),
                transform: TransformBundle::from_transform(Transform::from_translation(g_trans.translation())),
            },
        ));
    }
}

#[derive(Resource)]
struct ExplosionAudio(Vec<Handle<AudioSource>>);

pub(super) fn register(app: &mut App) {
    load_assets::<AudioSource, ExplosionAudio>(
        app,
        GameState::Loading,
        vec![
            "cosmos/sounds/sfx/explosion-1.ogg",
            "cosmos/sounds/sfx/explosion-2.ogg",
            "cosmos/sounds/sfx/explosion-3.ogg",
            "cosmos/sounds/sfx/explosion-4.ogg",
        ],
        |mut commands, sounds| {
            let sounds = sounds
                .into_iter()
                .flat_map(|(item, state)| match state {
                    LoadState::Loaded => Some(item),
                    _ => None,
                })
                .collect::<Vec<Handle<AudioSource>>>();

            commands.insert_resource(ExplosionAudio(sounds));
        },
    );

    app.add_systems(
        Update,
        respond_to_explosion
            .in_set(ExplosionSystemSet::ProcessExplosions)
            .run_if(in_state(GameState::Playing)),
    );
}
