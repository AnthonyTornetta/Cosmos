//! Manages the pilot of a ship

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    state::GameState,
    utils::ecs::register_fixed_update_removed_component,
};

/// A pilot component is bi-directional, if a player has the component then the entity it points to also has this component which points to the player.
#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct Pilot {
    /// This will either be the ship the player is piloting, or the pilot of the ship
    ///
    /// This value is dependent upon who has the component (structure gives pilot, player gives structure)
    pub entity: Entity,
}

#[derive(Component, Reflect, Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
/// The entity the pilot of this ship has focused. This component is set directly by the client, if
/// it is being piloted by them, and should not be trusted for validity!
///
/// This component will be present on the ship.
pub struct PilotFocused(pub Entity);

impl IdentifiableComponent for PilotFocused {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:pilot_focused"
    }
}

impl SyncableComponent for PilotFocused {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ClientAuthoritative(crate::netty::sync::ClientAuthority::Piloting)
    }

    #[cfg(feature = "client")]
    fn should_send_to_server(&self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> bool {
        mapping.server_from_client(&self.0).is_some()
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(&self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.server_from_client(&self.0).map(Self)
    }
}

fn remove_pilot_focused_on_no_pilot(mut commands: Commands, q_has_component: Query<Entity, (With<PilotFocused>, Without<Pilot>)>) {
    for e in q_has_component.iter() {
        commands.entity(e).remove::<PilotFocused>();
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<Pilot>();
    app.register_type::<PilotFocused>();

    sync_component::<PilotFocused>(app);
    register_fixed_update_removed_component::<Pilot>(app);

    app.add_systems(Update, remove_pilot_focused_on_no_pilot.run_if(in_state(GameState::Playing)));
}
