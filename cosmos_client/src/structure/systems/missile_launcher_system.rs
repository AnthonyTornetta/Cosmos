//! Client-side laser cannon system logic

use bevy::{asset::LoadState, prelude::*};
use bevy_kira_audio::prelude::*;
use cosmos_core::{physics::location::Location, structure::systems::missile_launcher_system::MissileLauncherSystem};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
};

use super::sync::sync_system;

#[derive(Event)]
/// This event is fired whenever a laser cannon system is fired
pub struct MissileLauncherSystemFiredEvent(pub Entity);

#[derive(Resource)]
struct MissileLauncherFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

fn apply_shooting_sound(
    query: Query<(&Location, &GlobalTransform)>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handles: Res<MissileLauncherFireHandles>,
    mut event_reader: EventReader<MissileLauncherSystemFiredEvent>,
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

struct MissileLauncherLoadingFlag;

// fn say_what_player_sees(
//     q_main_camera: Query<(&Transform, &GlobalTransform), With<MainCamera>>,
//     q_player: Query<&Location, With<LocalPlayer>>,
//     mut q_structures: Query<(&mut Structure, &Location, &GlobalTransform)>,
//     blocks: Res<Registry<Block>>,
//     inputs: InputChecker,
//     mut event_writer: EventWriter<BlockChangedEvent>,
// ) {
//     let Ok((cam_trans, cam_g_trans)) = q_main_camera.get_single() else {
//         return;
//     };

//     let Ok(p_loc) = q_player.get_single() else {
//         return;
//     };

//     for (mut structure, loc, structure_trans) in q_structures.iter_mut() {
//         let direction = cam_g_trans.affine().matrix3 * structure_trans.affine().inverse().matrix3;

//         let mut coords = vec![];
//         for coord in structure.raycast_iter(
//             cam_trans.translation + loc.relative_coords_to(p_loc),
//             direction * Vec3::NEG_Z,
//             10.0,
//             false,
//         ) {
//             // let block = structure.block_at(coord, &blocks);
//             // println!("Viewing block {}", block.unlocalized_name());

//             coords.push(coord);
//         }

//         if inputs.check_just_pressed(CosmosInputs::SymmetryZ) {
//             for coord in coords {
//                 structure.set_block_at(
//                     coord,
//                     blocks.from_id("cosmos:glass").unwrap(),
//                     Default::default(),
//                     &blocks,
//                     Some(&mut event_writer),
//                 );
//             }
//         }
//     }
// }

pub(super) fn register(app: &mut App) {
    sync_system::<MissileLauncherSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, MissileLauncherLoadingFlag>(
        app,
        GameState::PreLoading,
        vec!["cosmos/sounds/sfx/missile-launch-1.ogg", "cosmos/sounds/sfx/missile-launch-2.ogg"],
        |mut commands, handles| {
            commands.insert_resource(MissileLauncherFireHandles(
                handles.into_iter().filter(|x| x.1 == LoadState::Loaded).map(|x| x.0).collect(),
            ));
        },
    );

    app.add_event::<MissileLauncherSystemFiredEvent>().add_systems(
        Update,
        (/*say_what_player_sees,*/apply_shooting_sound).run_if(in_state(GameState::Playing)),
    );
}
