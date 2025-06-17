//! Player interactions with structures

use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    netty::{
        NettyChannelClient, client::LocalPlayer, client_unreliable_messages::ClientUnreliableMessages, cosmos_encoder,
    },
    state::GameState,
    structure::{ship::pilot::Pilot, systems::ShipActiveSystem},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::components::show_cursor::no_open_menus,
};

#[derive(Component, Default, Reflect)]
/// Contains the structure system's information currently hovered by the player
pub struct HoveredSystem {
    /// The index of the system, relative to the `active_systems` iterator
    pub hovered_system_index: usize,
    /// If the hovered system is active
    pub active: bool,
}

fn check_system_in_use(
    mut query: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>,
    input_handler: InputChecker,
    mut client: ResMut<RenetClient>,
) {
    let Ok(mut hovered_system) = query.single_mut() else {
        return;
    };

    hovered_system.active = input_handler.check_pressed(CosmosInputs::UseSelectedSystem);

    let active_system = if hovered_system.active {
        ShipActiveSystem::Active(hovered_system.hovered_system_index as u32)
    } else {
        ShipActiveSystem::Hovered(hovered_system.hovered_system_index as u32)
    };

    client.send_message(
        NettyChannelClient::Unreliable,
        cosmos_encoder::serialize(&ClientUnreliableMessages::ShipActiveSystem(active_system)),
    );
}

fn check_became_pilot(mut commands: Commands, query: Query<Entity, (Added<Pilot>, With<LocalPlayer>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(HoveredSystem::default());
    }
}

fn check_removed_pilot(mut commands: Commands, mut removed: RemovedComponents<Pilot>) {
    for ent in removed.read() {
        if let Ok(mut ecmds) = commands.get_entity(ent) {
            ecmds.remove::<HoveredSystem>();
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Used by the client to indicate which system they are currently activating
pub enum SystemUsageSet {
    /// The hovered slot component is added after becoming pilot
    AddHoveredSlotComponent,
    /// Used by the client to indicate which system they are currently activating
    ChangeSystemBeingUsed,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (SystemUsageSet::AddHoveredSlotComponent, SystemUsageSet::ChangeSystemBeingUsed).chain(),
    );

    app.add_systems(
        Update,
        (
            (check_system_in_use.run_if(no_open_menus), check_removed_pilot)
                .in_set(SystemUsageSet::ChangeSystemBeingUsed)
                .chain(),
            check_became_pilot.before(SystemUsageSet::AddHoveredSlotComponent),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<HoveredSystem>();
}
