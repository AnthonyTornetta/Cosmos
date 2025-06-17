//! Stops the server gracefully

use bevy::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    persistence::LoadingDistance,
    physics::{
        location::Location,
        player_world::PlayerWorld,
    },
};
use renet::RenetServer;

use crate::{
    commands::cosmos_command_handler::ProcessCommandsSet,
    persistence::saving::{NeedsSaved, SavingSystemSet},
};

#[derive(Debug, Event, Default)]
/// Tells the server to gracefully exit - saving all entities in the process.
pub struct StopServerEvent;

fn on_stop_server(
    mut commands: Commands,
    q_savable: Query<(Option<&Name>, &Location, Entity), (With<LoadingDistance>, Without<NeedsDespawned>, Without<PlayerWorld>)>,
    mut server: ResMut<RenetServer>,
    mut evw_close_after_save: EventWriter<CloseServerPostSaveEvent>,
) {
    info!("Received stop server event - Stopping server");

    info!("Disconnecting all players");
    server.disconnect_all();

    for (name, loc, ent) in q_savable.iter() {
        if let Some(name) = name {
            info!("Saving and unloading {name} ({ent:?})");
        } else {
            info!("Saving and unloading {ent:?} at {loc}");
        }

        commands.entity(ent).insert((NeedsSaved, NeedsDespawned));
    }

    evw_close_after_save.write_default();
}

fn shut_server_down(mut evw_app_exit: EventWriter<AppExit>) {
    info!("Shutting down server...");

    evw_app_exit.write_default();
}

#[derive(Event, Default)]
struct CloseServerPostSaveEvent;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The system sets the graceful server shutdown process follows
///
/// Note that these happen in different schedules
pub enum StopServerSet {
    /// Triggers the saving + despawn of all entities + disconnects all players
    ///
    /// Happens in the [`Update`] schedule
    Stop,
    /// Terminates the server process
    ///
    /// Happens in the [`First`] schedule after saving is finished.
    ShutDown,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, StopServerSet::Stop);
    app.configure_sets(First, StopServerSet::ShutDown.after(SavingSystemSet::DoneSaving));

    app.add_event::<StopServerEvent>()
        .add_event::<CloseServerPostSaveEvent>()
        .add_systems(
            FixedUpdate,
            on_stop_server
                .after(ProcessCommandsSet::HandleCommands)
                .in_set(StopServerSet::Stop)
                .run_if(on_event::<StopServerEvent>),
        )
        .add_systems(
            First,
            shut_server_down
                .in_set(StopServerSet::ShutDown)
                .run_if(on_event::<CloseServerPostSaveEvent>),
        );
}
