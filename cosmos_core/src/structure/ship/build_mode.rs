//! Handles build-mode functionality
//!
//! Note that build mode is currently only intended for ships, but is not yet manually limited to only ships.

use bevy::prelude::{App, BuildChildren, Commands, Component, Entity, Event, EventReader, Update};
use bevy_rapier3d::prelude::{RigidBodyDisabled, Sensor};

#[derive(Component, Default)]
/// Denotes that a player is in build mode
pub struct BuildMode;

#[derive(Event)]
/// This event is sent when a player is entering build mode
pub struct EnterBuildModeEvent {
    /// The player that's entering build mode
    pub player_entity: Entity,
    /// The structure they are entering build mode for
    ///
    /// Multiple players can be building on the same structure
    pub structure_entity: Entity,
}

#[derive(Event)]
/// This event is sent when a player is done being in build mode
pub struct ExitBuildModeEvent {
    /// The player done being in build mode
    pub player_entity: Entity,
}

fn enter_build_mode_listener(mut commands: Commands, mut event_reader: EventReader<EnterBuildModeEvent>) {
    for ev in event_reader.iter() {
        let Some(mut ecmds) = commands.get_entity(ev.player_entity) else {
            continue;
        };

        ecmds
            .insert(BuildMode::default())
            .insert(RigidBodyDisabled)
            .insert(Sensor)
            .set_parent(ev.structure_entity);
    }
}

fn exit_build_mode_listener(mut commands: Commands, mut event_reader: EventReader<ExitBuildModeEvent>) {
    for ev in event_reader.iter() {
        let Some(mut ecmds) = commands.get_entity(ev.player_entity) else {
            continue;
        };

        // Keep them as a child of the ship
        ecmds.remove::<BuildMode>().remove::<RigidBodyDisabled>().remove::<Sensor>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (enter_build_mode_listener, exit_build_mode_listener))
        .add_event::<EnterBuildModeEvent>()
        .add_event::<ExitBuildModeEvent>();
}
