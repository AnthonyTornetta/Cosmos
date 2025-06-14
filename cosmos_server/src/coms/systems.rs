use bevy::prelude::*;
use cosmos_core::{
    coms::{
        ComsChannel, ComsChannelType, ComsMessage, RequestedComs,
        events::{AcceptComsEvent, DeclineComsEvent, RequestCloseComsEvent, RequestComsEvent, SendComsMessage, SendComsMessageType},
    },
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::Location,
    prelude::{DespawnWithStructure, Ship},
    structure::ship::pilot::Pilot,
};

use super::{NpcRequestCloseComsEvent, NpcSendComsMessage, RequestHailFromNpc, RequestHailToNpc};

const MAX_HAIL_RANGE: f32 = 20_000.0;

fn on_request_coms(
    q_loc: Query<&Location>,
    mut nevr: EventReader<NettyEventReceived<RequestComsEvent>>,
    mut evr: EventReader<RequestHailFromNpc>,
    mut nevw_req: NettyEventWriter<RequestComsEvent>,
    mut evw_request_hail_npc: EventWriter<RequestHailToNpc>,
    q_player: Query<&Player>,
    lobby: Res<ServerLobby>,
    q_pilot: Query<(&Location, &Pilot)>,
    q_ship_loc: Query<&Location, With<Ship>>,
    q_coms: Query<(&ChildOf, &ComsChannel)>,
    q_requested_coms: Query<&RequestedComs>,
    mut commands: Commands,
) {
    for (requester_loc, this_ship_ent, other_ship_ent, coms_type) in nevr
        .read()
        .flat_map(|ev| {
            let (player_loc, pilot) = lobby.player_from_id(ev.client_id).and_then(|x| q_pilot.get(x).ok())?;

            Some((player_loc, pilot.entity, ev.event.0, ComsChannelType::Player))
        })
        .chain(evr.read().flat_map(|ev| {
            let loc = q_loc.get(ev.npc_ship).ok()?;

            Some((loc, ev.npc_ship, ev.player_ship, ComsChannelType::Ai(ev.ai_coms_type)))
        }))
    {
        let Ok(other_ship_loc) = q_ship_loc.get(other_ship_ent) else {
            info!("Not a ship");
            continue;
        };

        if !other_ship_loc.is_within_reasonable_range(requester_loc)
            || other_ship_loc.distance_sqrd(requester_loc) > MAX_HAIL_RANGE * MAX_HAIL_RANGE
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
            .any(|(com_parent, com)| com_parent.parent() == this_ship_ent && com.with == other_ship_ent)
        {
            warn!("Already an open channel");
            // There is already an open coms channel
            return;
        }

        if let Ok((_, pilot)) = q_pilot.get(other_ship_ent) {
            if let Ok(player) = q_player.get(pilot.entity) {
                info!("Requesting player hail.");
                nevw_req.write(RequestComsEvent(this_ship_ent), player.client_id());
                commands.entity(other_ship_ent).insert(RequestedComs {
                    coms_type: Some(coms_type),
                    from: this_ship_ent,
                    time: 0.0,
                });
            } else {
                info!("Requesting NPC hail.");
                evw_request_hail_npc.write(RequestHailToNpc {
                    player_ship: this_ship_ent,
                    npc_ship: other_ship_ent,
                });
                commands.entity(other_ship_ent).insert(RequestedComs {
                    coms_type: None,
                    from: this_ship_ent,
                    time: 0.0,
                });
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
        let Some((player_loc, pilot)) = lobby.player_from_id(ev.client_id).and_then(|x| q_pilot.get(x).ok()) else {
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

        let channel_type = req_coms.coms_type.unwrap_or(ComsChannelType::Player);

        commands.entity(this_ship_ent).remove::<RequestedComs>().with_children(|p| {
            p.spawn((
                DespawnWithStructure,
                ComsChannel {
                    with: other_ship_ent,
                    messages: vec![],
                    channel_type,
                },
            ));
        });

        commands.entity(other_ship_ent).with_children(|p| {
            p.spawn((
                DespawnWithStructure,
                ComsChannel {
                    with: this_ship_ent,
                    messages: vec![],
                    channel_type,
                },
            ));
        });
    }
}

fn tick_requested_coms(mut commands: Commands, time: Res<Time>, mut q_req_coms: Query<(Entity, &mut RequestedComs)>) {
    const MAX_SECS: f32 = 1000.0;

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
    mut evr_send_coms: EventReader<NpcSendComsMessage>,
    mut q_coms: Query<(&ChildOf, &mut ComsChannel)>,
) {
    for (from, message, to) in nevr_com_msg
        .read()
        .flat_map(|ev| {
            let player_ent = lobby.player_from_id(ev.client_id)?;
            let pilot = q_pilot.get(player_ent).ok()?;

            Some((pilot.entity, ev.message.clone(), ev.to))
        })
        .chain(
            evr_send_coms
                .read()
                .map(|ev| (ev.from_ship, SendComsMessageType::Message(ev.message.to_owned()), ev.to_ship)),
        )
    {
        let Some((_, mut coms)) = q_coms.iter_mut().find(|(parent, coms)| parent.parent() == from && coms.with == to) else {
            warn!("(1) No coms entry! to: {:?} | ship = {:?}", to, from);
            continue;
        };

        let msg = ComsMessage {
            sender: from,
            text: match &message {
                SendComsMessageType::Message(s) => s.into(),
                SendComsMessageType::Yes => "Yes".into(),
                SendComsMessageType::No => "No".into(),
            },
        };

        coms.messages.push(msg.clone());

        let Some((_, mut coms)) = q_coms.iter_mut().find(|(parent, coms)| parent.parent() == to && coms.with == from) else {
            warn!("(2) No coms entry! to: {:?} | ship = {:?}", to, from);
            continue;
        };

        coms.messages.push(msg);
    }
}

fn ensure_coms_still_active(mut commands: Commands, q_coms: Query<(Entity, &ComsChannel, &ChildOf)>) {
    for (ent, coms_channel, parent) in q_coms.iter() {
        if q_coms
            .iter()
            .any(|(_, c, p)| c.with == parent.parent() && p.get() == coms_channel.with)
        {
            continue;
        }

        // The coms channel this points to has been terminated
        commands.entity(ent).insert(NeedsDespawned);
    }
}

fn on_req_close_coms(
    lobby: Res<ServerLobby>,
    q_pilot: Query<&Pilot>,
    q_parent: Query<&ChildOf>,
    mut nevr_close_coms: EventReader<NettyEventReceived<RequestCloseComsEvent>>,
    mut npc_close_coms: EventReader<NpcRequestCloseComsEvent>,
    q_coms: Query<(Entity, &ComsChannel)>,
    mut commands: Commands,
) {
    for (this_ship, coms_ent) in nevr_close_coms
        .read()
        .flat_map(|ev| {
            let player = lobby.player_from_id(ev.client_id)?;
            let pilot = q_pilot.get(player).ok()?;

            Some((pilot.entity, ev.0))
        })
        .chain(npc_close_coms.read().map(|ev| (ev.npc_ship, ev.coms_entity)))
    {
        let Ok((coms_ent, coms)) = q_coms.get(coms_ent) else {
            warn!("Invalid coms ent - {:?}", coms_ent);
            continue;
        };

        let Ok(my_ship_ent) = q_parent.get(coms_ent) else {
            warn!("Invalid coms heirarchy");
            continue;
        };

        if my_ship_ent.get() != this_ship {
            warn!("No authority to close this coms!");
            continue;
        }

        let coms_parent = my_ship_ent.get();

        let Some((other_coms_ent, _)) = q_coms
            .iter()
            .find(|(ent, x)| x.with == coms_parent && q_parent.get(*ent).expect("Invalid coms heirarchy").get() == coms.with)
        else {
            warn!("Unable to find coms.");
            continue;
        };

        info!("Removing coms");
        commands.entity(other_coms_ent).insert(NeedsDespawned);
        commands.entity(coms_ent).insert(NeedsDespawned);
    }
}

fn on_decline_coms(
    mut commands: Commands,
    lobby: Res<ServerLobby>,
    mut nevr_decline_coms: EventReader<NettyEventReceived<DeclineComsEvent>>,
    q_piloting: Query<&Pilot>,
) {
    for ev in nevr_decline_coms.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok(pilot) = q_piloting.get(player) else {
            continue;
        };

        commands.entity(pilot.entity).remove::<RequestedComs>();
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
            on_req_close_coms,
            on_decline_coms,
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between),
    );
}
