use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};
use bevy::prelude::*;
use cosmos_core::coms::ComsChannel;
use cosmos_core::coms::events::{AcceptComsEvent, RequestComsEvent};
use cosmos_core::netty::client::LocalPlayer;
use cosmos_core::netty::sync::events::client_event::NettyEventWriter;
use cosmos_core::netty::system_sets::NetworkingSystemsSet;
use cosmos_core::prelude::Ship;
use cosmos_core::structure::ship::pilot::{Pilot, PilotFocused};

use super::ui::coms_request::OpenRequestComsUi;

fn initiate_coms_request(
    q_ship: Query<(), With<Ship>>,
    inputs: InputChecker,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_local_pilot: Query<&Pilot, With<LocalPlayer>>,
    q_focused: Query<&PilotFocused>,
    q_coms: Query<(&ChildOf, &ComsChannel)>,
    mut nevw_request_coms: NettyEventWriter<RequestComsEvent>,
) {
    let Ok(pilot) = q_local_pilot.single() else {
        return;
    };

    let Ok(pilot_focused) = q_focused.get(pilot.entity) else {
        return;
    };

    if !q_ship.contains(pilot_focused.0) {
        return;
    }

    if !inputs.check_just_pressed(CosmosInputs::HailShip) {
        return;
    }

    let lp = q_local_player.single().expect("Local player missing");

    let mut all_coms = q_coms.iter().filter(|(parent, _)| parent.parent() == lp);

    if all_coms.any(|(parent, coms)| coms.with == pilot.entity && parent.parent() == pilot_focused.0) {
        info!("Already in coms w/ this ship.");
        return;
    }

    info!("Sending coms request!");
    nevw_request_coms.write(RequestComsEvent(pilot_focused.0));
}

fn read_coms_request(
    q_local_player: Query<Entity, With<LocalPlayer>>,
    q_local_pilot: Query<&Pilot, With<LocalPlayer>>,
    q_coms: Query<(&ChildOf, &ComsChannel)>,
    mut nevr_request_coms: EventReader<RequestComsEvent>,
    mut nevw_accept_coms: NettyEventWriter<AcceptComsEvent>,
    mut evw_open_req_coms_ui: EventWriter<OpenRequestComsUi>,
) {
    for ev in nevr_request_coms.read() {
        let Ok(pilot) = q_local_pilot.single() else {
            return;
        };

        info!("Got coms req!");

        let requester = ev.0;

        let lp = q_local_player.single().expect("Local player missing");

        let mut all_coms = q_coms.iter().filter(|(parent, _)| parent.parent() == lp);

        if all_coms.any(|(parent, coms)| coms.with == requester && parent.parent() == pilot.entity) {
            info!("There is already an active coms session with this ship - auto-accepting coms.");
            nevw_accept_coms.write(AcceptComsEvent(requester));
            return;
        }

        evw_open_req_coms_ui.write(OpenRequestComsUi(requester));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (initiate_coms_request, read_coms_request));
}
