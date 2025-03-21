use bevy::prelude::*;
use cosmos_core::{
    coms::{
        events::{AcceptComsEvent, RequestComsEvent, SendComsMessage},
        ComsChannel, ComsMessage, RequestedComs,
    },
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    prelude::Ship,
    structure::ship::pilot::Pilot,
};

use super::RequestHailNpc;

const MAX_HAIL_RANGE: f32 = 20_000.0;

fn on_request_coms(
    mut nevr: EventReader<NettyEventReceived<RequestComsEvent>>,
    mut nevw_req: NettyEventWriter<RequestComsEvent>,
    mut evw_request_hail_npc: EventWriter<RequestHailNpc>,
    q_player: Query<&Player>,
    lobby: Res<ServerLobby>,
    q_pilot: Query<(&Location, &Pilot)>,
    q_ship_loc: Query<&Location, With<Ship>>,
    q_coms: Query<(&Parent, &ComsChannel)>,
    q_requested_coms: Query<&RequestedComs>,
    mut commands: Commands,
) {
    for ev in nevr.read() {
        let Some((player_loc, pilot)) = lobby.player_from_id(ev.client_id).map(|x| q_pilot.get(x).ok()).flatten() else {
            continue;
        };

        let this_ship_ent = pilot.entity;
        let other_ship_ent = ev.event.0;

        let Ok(other_ship_loc) = q_ship_loc.get(other_ship_ent) else {
            info!("Not a ship");
            continue;
        };

        if !other_ship_loc.is_within_reasonable_range(player_loc)
            || other_ship_loc.distance_sqrd(player_loc) > MAX_HAIL_RANGE * MAX_HAIL_RANGE
        {
            info!("Too far!");
            return;
        }

        if q_requested_coms.contains(other_ship_ent) {
            info!("Already being requesed!");
            // Someone already requested them
            return;
        }

        if q_coms
            .iter()
            .any(|(com_parent, com)| com_parent.get() == this_ship_ent && com.with == other_ship_ent)
        {
            warn!("Already an open channel");
            // There is already an open coms channel
            return;
        }

        info!("Requested coms!");
        commands.entity(other_ship_ent).insert(RequestedComs {
            from: this_ship_ent,
            time: 0.0,
        });

        if let Ok((_, pilot)) = q_pilot.get(other_ship_ent) {
            if let Ok(player) = q_player.get(pilot.entity) {
                info!("Requesting other player hail.");
                nevw_req.send(RequestComsEvent(this_ship_ent), player.client_id());
            } else {
                info!("Requesting NPC hail.");
                evw_request_hail_npc.send(RequestHailNpc { player: this_ship_ent });
            }
        } else {
            info!("TODO: Let everyone on ship know they are being hailed!");
        }
    }
}

fn on_accept_coms(
    lobby: Res<ServerLobby>,
    q_pilot: Query<(&Location, &Pilot)>,
    q_requested_coms: Query<&RequestedComs>,
    mut commands: Commands,
    mut nevr_accept_coms: EventReader<NettyEventReceived<AcceptComsEvent>>,
) {
    for ev in nevr_accept_coms.read() {
        let Some((player_loc, pilot)) = lobby.player_from_id(ev.client_id).map(|x| q_pilot.get(x).ok()).flatten() else {
            info!("Not a pilot player");
            continue;
        };

        let this_ship_ent = pilot.entity;
        let other_ship_ent = ev.event.0;

        let Ok((other_ship_loc, _)) = q_pilot.get(other_ship_ent) else {
            warn!("Bad entity ({other_ship_ent:?})");
            continue;
        };

        let Ok(req_coms) = q_requested_coms.get(this_ship_ent) else {
            continue;
        };

        if req_coms.from != ev.event.0 {
            info!("Accepted coms from someone that didn't request it.");
            continue;
        }

        if !other_ship_loc.is_within_reasonable_range(player_loc)
            || other_ship_loc.distance_sqrd(player_loc) > MAX_HAIL_RANGE * MAX_HAIL_RANGE
        {
            info!("Accepted something that's too far.");
            return;
        }

        info!("Inserting coms components!");

        commands.entity(this_ship_ent).remove::<RequestedComs>().with_children(|p| {
            p.spawn((ComsChannel {
                with: other_ship_ent,
                messages: vec![],
            },));
        });

        commands.entity(other_ship_ent).with_children(|p| {
            p.spawn((ComsChannel {
                with: this_ship_ent,
                messages: vec![],
            },));
        });
    }
}

fn tick_requested_coms(mut commands: Commands, time: Res<Time>, mut q_req_coms: Query<(Entity, &mut RequestedComs)>) {
    const MAX_SECS: f32 = 15.0;

    for (ent, mut req_com) in q_req_coms.iter_mut() {
        req_com.time += time.delta_secs();

        if req_com.time > MAX_SECS {
            commands.entity(ent).remove::<RequestedComs>();
        }
    }
}

fn send_coms_message(
    lobby: Res<ServerLobby>,
    q_pilot: Query<&Pilot>,
    mut nevr_com_msg: EventReader<NettyEventReceived<SendComsMessage>>,
    mut q_coms: Query<(&Parent, &mut ComsChannel)>,
) {
    for ev in nevr_com_msg.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok(pilot) = q_pilot.get(player_ent) else {
            continue;
        };

        let Some((_, mut coms)) = q_coms
            .iter_mut()
            .find(|(parent, coms)| parent.get() == pilot.entity && coms.with == ev.to)
        else {
            warn!("No coms entry!");
            continue;
        };

        coms.messages.push(ComsMessage {
            text: ev.event.message.clone(),
        });

        let Some((_, mut coms)) = q_coms
            .iter_mut()
            .find(|(parent, coms)| parent.get() == ev.to && coms.with == pilot.entity)
        else {
            warn!("No coms entry!");
            continue;
        };

        coms.messages.push(ComsMessage {
            text: ev.event.message.clone(),
        });
    }
}

fn ensure_coms_still_active(mut commands: Commands, q_coms: Query<(Entity, &ComsChannel, &Parent)>) {
    for (ent, coms_channel, parent) in q_coms.iter() {
        if q_coms
            .iter()
            .any(|(_, c, p)| c.with == parent.get() && p.get() == coms_channel.with)
        {
            continue;
        }

        // The coms channel this points to has been terminated
        commands.entity(ent).insert(NeedsDespawned);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_request_coms,
            on_accept_coms,
            tick_requested_coms,
            ensure_coms_still_active,
            send_coms_message,
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    );
}
