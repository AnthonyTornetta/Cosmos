//! Handles build-mode functionality
//!
//! Note that build mode is currently only intended for ships, but is not yet manually limited to only ships.

use bevy::{
    math::Quat,
    prelude::{
        Added, App, BuildChildrenTransformExt, Changed, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs,
        IntoSystemSetConfigs, Parent, Query, SystemSet, Transform, Update, With, Without,
    },
    reflect::Reflect,
};
use bevy_rapier3d::{
    dynamics::RigidBody,
    prelude::{RigidBodyDisabled, Sensor},
};
use serde::{Deserialize, Serialize};

use crate::{
    block::block_events::BlockEventsSet, netty::system_sets::NetworkingSystemsSet, prelude::StructureBlock,
    structure::coordinates::CoordinateType,
};

type BuildModeSymmetries = (Option<CoordinateType>, Option<CoordinateType>, Option<CoordinateType>);

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone, Copy)]
/// Denotes that a player is in build mode
///
/// The player's parent will be the structure they are building
pub struct BuildMode {
    symmetries: BuildModeSymmetries,
    block: StructureBlock,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
/// Represents the X/Y/Z symmetry axis
pub enum BuildAxis {
    /// X axis
    X,
    /// Y axis
    Y,
    /// Z axis
    Z,
}

impl BuildMode {
    fn internal_set_symmetry(&mut self, axis: BuildAxis, coordinate: Option<CoordinateType>) {
        match axis {
            BuildAxis::X => self.symmetries.0 = coordinate,
            BuildAxis::Y => self.symmetries.1 = coordinate,
            BuildAxis::Z => self.symmetries.2 = coordinate,
        }
    }

    /// Sets the symmetry for this axis
    pub fn set_symmetry(&mut self, axis: BuildAxis, coordinate: CoordinateType) {
        self.internal_set_symmetry(axis, Some(coordinate));
    }

    /// Removes the symmetry from this axis
    pub fn remove_symmetry(&mut self, axis: BuildAxis) {
        self.internal_set_symmetry(axis, None);
    }

    /// Gets the symmetry for this axis - `None` if there is no symmetry present.
    pub fn get_symmetry(&self, axis: BuildAxis) -> Option<CoordinateType> {
        match axis {
            BuildAxis::X => self.symmetries.0,
            BuildAxis::Y => self.symmetries.1,
            BuildAxis::Z => self.symmetries.2,
        }
    }
}

#[derive(Event)]
/// This event is sent when a player is entering build mode
pub struct EnterBuildModeEvent {
    /// The player that's entering build mode
    pub player_entity: Entity,
    /// The structure they are entering build mode for
    ///
    /// Multiple players can be building on the same structure
    pub structure_entity: Entity,
    /// The block containing the build mode block
    pub block: StructureBlock,
}

#[derive(Event)]
/// This event is sent when a player is done being in build mode
pub struct ExitBuildModeEvent {
    /// The player done being in build mode
    pub player_entity: Entity,
}

fn enter_build_mode_listener(mut commands: Commands, mut event_reader: EventReader<EnterBuildModeEvent>) {
    for ev in event_reader.read() {
        let Some(mut ecmds) = commands.get_entity(ev.player_entity) else {
            continue;
        };

        ecmds
            .insert(BuildMode {
                block: ev.block,
                symmetries: Default::default(),
            })
            .insert(RigidBodyDisabled)
            .insert(RigidBody::Fixed)
            .insert(Sensor)
            .set_parent_in_place(ev.structure_entity);
    }
}

fn exit_build_mode_listener(mut commands: Commands, mut event_reader: EventReader<ExitBuildModeEvent>) {
    for ev in event_reader.read() {
        let Some(mut ecmds) = commands.get_entity(ev.player_entity) else {
            continue;
        };

        // Keep them as a child of the ship
        ecmds
            .remove::<BuildMode>()
            .remove::<RigidBodyDisabled>()
            .remove::<Sensor>()
            .insert(RigidBody::Dynamic)
            .remove::<InBuildModeFlag>();
    }
}

#[derive(Component)]
struct InBuildModeFlag;

fn exit_build_mode_when_parent_dies(
    query: Query<Entity, (With<BuildMode>, Without<Parent>)>,
    changed_query: Query<(Entity, Option<&InBuildModeFlag>), (With<BuildMode>, Changed<Parent>)>,
    mut event_writer: EventWriter<ExitBuildModeEvent>,
    mut commands: Commands,
) {
    for entity in query.iter() {
        event_writer.send(ExitBuildModeEvent { player_entity: entity });
    }

    for (entity, in_build_mode) in changed_query.iter() {
        if in_build_mode.is_some() {
            event_writer.send(ExitBuildModeEvent { player_entity: entity });
        } else {
            commands.entity(entity).insert(InBuildModeFlag);
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Build mode interactions
pub enum BuildModeSet {
    /// When the player attempts to enter build mode, their event will be sent here
    SendEnterBuildModeEvent,
    /// The player will enter build mode
    EnterBuildMode,
    /// When the player attempts to exit build mode, their event will be sent here
    SendExitBuildModeEvent,
    /// The player will exit build mode
    ExitBuildMode,
}

fn adjust_transform_build_mode(mut q_transform: Query<&mut Transform, Added<BuildMode>>) {
    for mut trans in q_transform.iter_mut() {
        trans.rotation = Quat::IDENTITY;
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            BuildModeSet::SendEnterBuildModeEvent,
            BuildModeSet::EnterBuildMode,
            BuildModeSet::SendExitBuildModeEvent,
            BuildModeSet::ExitBuildMode,
        )
            .chain(),
    );

    app.add_systems(
        Update,
        (
            (enter_build_mode_listener, adjust_transform_build_mode).in_set(BuildModeSet::EnterBuildMode),
            exit_build_mode_when_parent_dies.in_set(BuildModeSet::SendExitBuildModeEvent),
            exit_build_mode_listener.in_set(BuildModeSet::ExitBuildMode),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockEventsSet::ProcessEvents),
    )
    .add_event::<EnterBuildModeEvent>()
    .add_event::<ExitBuildModeEvent>()
    .register_type::<BuildMode>();
}
