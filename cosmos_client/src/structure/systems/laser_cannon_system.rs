use bevy::prelude::*;

// fn prepare_sound(
//     asset_server: Res<AssetServer>,
//     mut commands: Commands,
//     mut loader: ResMut<LoadingManager>,
//     mut event_writer: EventWriter<AddLoadingEvent>,
// ) {
//     let id = loader.register_loader(&mut event_writer);

//     commands.insert_resource(LoadingAudioHandle(asset_server.load("cosmos/sounds/sfx/thruster-running.ogg"), id));
// }

// fn check_sound_done_loading(
//     handle: Option<Res<LoadingAudioHandle>>,
//     asset_server: Res<AssetServer>,
//     mut commands: Commands,
//     mut loader: ResMut<LoadingManager>,
//     mut end_writer: EventWriter<DoneLoadingEvent>,
// ) {
//     if let Some(handle) = handle {
//         if asset_server.get_load_state(handle.0.id()) == LoadState::Loaded {
//             commands.insert_resource(ThrusterAudioHandle(handle.0.clone()));
//             commands.remove_resource::<LoadingAudioHandle>();

//             loader.finish_loading(handle.1, &mut end_writer);
//         }
//     }
// }

pub(super) fn register(_app: &mut App) {}
