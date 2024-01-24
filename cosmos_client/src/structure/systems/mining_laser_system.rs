//! Client-side laser cannon system logic

use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::structure::systems::{
    energy_storage_system::EnergyStorageSystem, mining_laser_system::MiningLaserSystem, StructureSystem, SystemActive, Systems,
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
struct LaserCannonFireHandles(Vec<Handle<AudioSource>>);

fn apply_mining_sound(
    q_systems: Query<&Systems>,
    q_mining_lasers: Query<(&StructureSystem, &MiningLaserSystem), With<SystemActive>>,
    q_energy_storage_system: Query<&EnergyStorageSystem>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handles: Res<LaserCannonFireHandles>,
) {
    for (structure_system, mining_laser_system) in q_mining_lasers.iter() {
        if mining_laser_system.lines.is_empty() {
            continue;
        }

        let structure_entity = structure_system.structure_entity();

        let Ok(systems) = q_systems.get(structure_entity) else {
            error!("Missing systems for ship {:?}", structure_entity);
            continue;
        };

        let Ok(energy_storage_system) = systems.query(&q_energy_storage_system) else {
            error!("Ship missing energy storage system! {:?}", structure_entity);
            continue;
        };

        // Not an amazing way to check if there is enough energy, will refactor once you have to wire everything up.
        if energy_storage_system.get_energy() <= f32::EPSILON {
            continue;
        }

        let idx = rand::random::<usize>() % audio_handles.0.len();

        let playing_sound: Handle<AudioInstance> = audio.play(audio_handles.0[idx].clone()).handle();

        commands.entity(structure_entity).with_children(|p| {
            p.spawn((
                CosmosAudioEmitter {
                    emissions: vec![AudioEmission {
                        instance: playing_sound,
                        peak_volume: 0.3,
                        ..Default::default()
                    }],
                },
                DespawnOnNoEmissions,
                TransformBundle::from_transform(Transform::from_xyz(0.5, 0.5, 1.0)),
            ));
        });
    }
}

struct LaserCannonLoadingFlag;

pub(super) fn register(app: &mut App) {
    sync_system::<MiningLaserSystem>(app);

    load_assets::<AudioSource, LaserCannonLoadingFlag>(
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

    app.add_event::<LaserCannonSystemFiredEvent>()
        .add_systems(Update, apply_mining_sound.run_if(in_state(GameState::Playing)));
}
