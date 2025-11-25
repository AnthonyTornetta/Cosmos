use bevy::{
    app::{App, Startup},
    camera::Camera3d,
    ecs::system::Commands,
};
// use iyes_perf_ui::{
//     entries::{PerfUiFramerateEntries, PerfUiSystemEntries},
//     prelude::*,
// };
//
fn create_debug(mut commands: Commands) {
    //     commands.spawn((
    //         PerfUiRoot::default(),
    //         // Contains everything related to FPS and frame time
    //         PerfUiFramerateEntries::default(),
    //         PerfUiEntryEntityCount::default(),
    //         // Contains everything related to system diagnostics (CPU, RAM)
    //         PerfUiSystemEntries::default(),
    //     ));
    //
    // commands.spawn(Camera3d { ..Default::default() });
}
//
pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, create_debug);
}
