use bevy::{
    app::Update,
    log::warn,
    prelude::{in_state, App, Event, EventWriter, IntoSystemConfigs, ResMut},
};
use cosmos_core::{
    netty::{client_registry::RegistrySyncing, cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    state::GameState,
};
use renet2::{ClientId, RenetServer};

#[derive(Debug, Event)]
pub struct ClientFinishedReceivingRegistriesEvent(pub ClientId);

fn listen_for_done_syncing(
    mut server: ResMut<RenetServer>,
    mut evw_finished_receiving_registries: EventWriter<ClientFinishedReceivingRegistriesEvent>,
) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::ComponentReplication) {
            let Ok(msg) = cosmos_encoder::deserialize::<RegistrySyncing>(&message) else {
                warn!("Bad deserialization");
                continue;
            };

            match msg {
                RegistrySyncing::FinishedReceivingRegistries => {
                    evw_finished_receiving_registries.send(ClientFinishedReceivingRegistriesEvent(client_id));
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        listen_for_done_syncing
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    )
    .add_event::<ClientFinishedReceivingRegistriesEvent>();
}
