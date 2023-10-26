//! Handles build-mode functionality
//!
//! Note that build mode is currently only intended for ships, but is not yet manually limited to only ships.

use bevy::{
    prelude::{
        App, BuildChildren, Changed, Commands, Component, Entity, Event, EventReader, EventWriter, Parent, Query, Update, With, Without,
    },
    reflect::Reflect,
};
use bevy_rapier3d::prelude::{RigidBodyDisabled, Sensor};
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::CoordinateType;

type BuildModeSymmetries = (Option<CoordinateType>, Option<CoordinateType>, Option<CoordinateType>);

#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize, Clone, Copy)]
/// Denotes that a player is in build mode
///
/// The player's parent will be the structure they are building
pub struct BuildMode {
    symmetries: BuildModeSymmetries,
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
        ecmds
            .remove::<BuildMode>()
            .remove::<RigidBodyDisabled>()
            .remove::<Sensor>()
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

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            enter_build_mode_listener,
            exit_build_mode_when_parent_dies,
            exit_build_mode_listener,
        ),
    )
    .add_event::<EnterBuildModeEvent>()
    .add_event::<ExitBuildModeEvent>()
    .register_type::<BuildMode>();
}
