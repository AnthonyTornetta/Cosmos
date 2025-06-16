use bevy::prelude::*;
use cosmos_core::{
    chat::{ClientSendChatMessageEvent, ServerSendChatMessageEvent},
    entities::player::Player,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

fn receive_messages(
    mut nevw_send_chat_msg: NettyEventWriter<ServerSendChatMessageEvent>,
    mut nevr_chat_msg: EventReader<NettyEventReceived<ClientSendChatMessageEvent>>,
    clients: Res<ServerLobby>,
    q_player: Query<&Player>,
) {
    for ev in nevr_chat_msg.read() {
        let Some(Ok((player_ent, player))) = clients
            .player_from_id(ev.client_id)
            .map(|player_ent| q_player.get(player_ent).map(|player| (player_ent, player)))
        else {
            continue;
        };

        match &ev.event {
            ClientSendChatMessageEvent::Global(msg) => {
                let message = format!("{}> {}", player.name(), msg);

                info!("{message}");

                nevw_send_chat_msg.broadcast(ServerSendChatMessageEvent {
                    sender: Some(player_ent),
                    message,
                });
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_messages
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
