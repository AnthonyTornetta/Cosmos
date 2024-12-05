use bevy::{
    app::App,
    prelude::{Camera3dBundle, Commands, OnEnter},
};
// use cosmos_core::state::GameState;
// use iyes_perf_ui::{
//     entries::{PerfUiFramerateEntries, PerfUiSystemEntries},
//     prelude::*,
// };
//
// fn create_debug(mut commands: Commands) {
//     commands.spawn((
//         PerfUiRoot::default(),
//         // Contains everything related to FPS and frame time
//         PerfUiFramerateEntries::default(),
//         PerfUiEntryEntityCount::default(),
//         // Contains everything related to system diagnostics (CPU, RAM)
//         PerfUiSystemEntries::default(),
//     ));
//
//     commands.spawn(Camera3dBundle { ..Default::default() });
// }
//
pub(super) fn register(app: &mut App) {
    // app.add_systems(OnEnter(GameState::Playing), create_debug);
}
