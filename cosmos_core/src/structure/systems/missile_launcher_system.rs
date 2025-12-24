//! Represents all the missile launchers on this structure

use std::time::Duration;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{
    ClientAuthority, IdentifiableComponent, SyncableComponent,
    events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
    sync_component,
};

use super::{
    StructureSystemsSet,
    line_system::{LineProperty, LinePropertyCalculator, LineSystem},
    sync::SyncableSystem,
};

/// A ship system that stores information about the missile cannons
///
/// See [`SystemCooldown`] for the missile cannon's duration
pub type MissileLauncherSystem = LineSystem<MissileLauncherProperty, MissileLauncherCalculator>;

impl SyncableSystem for MissileLauncherSystem {}

#[derive(Default, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// Every block that is a missile cannon should have this property
pub struct MissileLauncherProperty {
    /// How much energy is consumed per shot
    pub energy_per_shot: f32,
}

impl LineProperty for MissileLauncherProperty {}

#[derive(Debug, Reflect)]
/// Used internally by missile cannon system, but must be public for compiler to be happy.
///
/// A simple strategy pattern that is never initialized
pub struct MissileLauncherCalculator;

impl LinePropertyCalculator<MissileLauncherProperty> for MissileLauncherCalculator {
    fn calculate_property(properties: &[MissileLauncherProperty]) -> MissileLauncherProperty {
        properties
            .iter()
            .copied()
            .reduce(|a, b| MissileLauncherProperty {
                energy_per_shot: a.energy_per_shot + b.energy_per_shot,
            })
            .unwrap_or_default()
    }

    fn unlocalized_name() -> &'static str {
        "cosmos:missile_launcher_system"
    }
}

#[derive(Debug, Serialize, Deserialize, Component, Clone, Copy, Default, Reflect, PartialEq, Eq)]
/// Tracks how long the current target has been targetted by the missile system.
pub enum MissileLauncherFocus {
    #[default]
    /// The missile launcher is not focusing anything
    NotFocusing,
    /// The missile launcher is focusing something
    Focusing {
        /// The **SERVER** entity that is being focused. Even on the client, this
        /// will represent the server's representation of this entity. This is to make
        /// syncing this with the server easier.
        focusing_server_entity: Entity,
        /// How long the focusing has been happening.
        ///
        /// Capped to [`Self::complete_duration`]
        focused_duration: Duration,
        /// The maximum amount the missile launcher can focus for
        /// before it's ready to track that target.
        complete_duration: Duration,
    },
}

impl MissileLauncherFocus {
    /// Changes the missile focus to not focus on anything
    pub fn clear_focus(&mut self) {
        *self = MissileLauncherFocus::NotFocusing;
    }

    /// Changes the focus of the missile system.
    ///
    /// This does NOT check if it's already focused on the target, and will clear any currently focused
    /// progress.
    pub fn change_focus(&mut self, target: Entity, max_focus_duration: Duration) {
        *self = MissileLauncherFocus::Focusing {
            focusing_server_entity: target,
            focused_duration: Duration::default(),
            complete_duration: max_focus_duration,
        };
    }

    /// Returns the entity this is locked onto, if any.
    ///
    /// The focus duration must be equal to or exceed the compelte duration to be locked on.
    pub fn locked_on_to(&self) -> Option<Entity> {
        match *self {
            MissileLauncherFocus::Focusing {
                focusing_server_entity,
                focused_duration,
                complete_duration,
            } => {
                if focused_duration >= complete_duration {
                    Some(focusing_server_entity)
                } else {
                    None
                }
            }
            MissileLauncherFocus::NotFocusing => None,
        }
    }
}

impl IdentifiableComponent for MissileLauncherFocus {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:missile_launcher_focus"
    }
}

impl SyncableComponent for MissileLauncherFocus {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Component, Clone, Copy, Reflect, PartialEq, Eq)]
/// Prefers focusing this entity if there are many potential candidates
pub struct MissileLauncherPreferredFocus {
    /// The **SERVER** entity that is being focused. Even on the client, this
    /// will represent the server's representation of this entity. This is to make
    /// syncing this with the server easier.
    pub focusing_server_entity: Option<Entity>,
}

impl IdentifiableComponent for MissileLauncherPreferredFocus {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:missile_launcher_preferred_focus"
    }
}

impl SyncableComponent for MissileLauncherPreferredFocus {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ClientAuthoritative(ClientAuthority::Piloting)
    }
}

fn add_focus_to_new_missile_system(mut commands: Commands, q_added_missile_launcher_system: Query<Entity, Added<MissileLauncherSystem>>) {
    for ent in &q_added_missile_launcher_system {
        commands
            .entity(ent)
            .insert((MissileLauncherFocus::default(), MissileLauncherPreferredFocus::default()));
    }
}

fn name_missile_launcher_system(mut commands: Commands, q_added: Query<Entity, Added<MissileLauncherSystem>>) {
    for e in q_added.iter() {
        commands.entity(e).insert(Name::new("Missile Launcher System"));
    }
}

#[derive(Message, Serialize, Deserialize, Debug, Clone)]
/// Anything that can go wrong when firing a missile launcher system
pub enum MissileSystemFailure {
    /// The system has no more ammo (missile items) to pull from
    NoAmmo,
}

impl IdentifiableMessage for MissileSystemFailure {
    fn unlocalized_name() -> &'static str {
        "cosmos:missile_system_failure"
    }
}

impl NettyMessage for MissileSystemFailure {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<MissileLauncherPreferredFocus>(app);
    sync_component::<MissileLauncherFocus>(app);

    app.add_systems(
        FixedUpdate,
        add_focus_to_new_missile_system.after(StructureSystemsSet::UpdateSystems),
    )
    .register_type::<MissileLauncherPreferredFocus>()
    .register_type::<MissileLauncherFocus>()
    .add_systems(
        FixedUpdate,
        name_missile_launcher_system
            .ambiguous_with_all() // doesn't matter if this is 1-frame delayed
            .after(StructureSystemsSet::InitSystems),
    )
    .add_netty_message::<MissileSystemFailure>();
}
