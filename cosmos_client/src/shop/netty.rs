use bevy::{
    app::{App, Update},
    ecs::{
        event::EventWriter,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::ResMut,
    },
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::mut_events::MutEvent,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelServer},
    shop::netty::ServerShopMessages,
};

use crate::state::game_state::GameState;

use super::ui::OpenShopUiEvent;

fn listen_netty(mut client: ResMut<RenetClient>, mut ev_writer: EventWriter<MutEvent<OpenShopUiEvent>>) {
    while let Some(message) = client.receive_message(NettyChannelServer::Shop) {
        let msg: ServerShopMessages = cosmos_encoder::deserialize(&message).expect("Bad shop message");

        match msg {
            ServerShopMessages::OpenShop {
                shop_block,
                structure_entity,
                shop_data,
            } => {
                ev_writer.send(OpenShopUiEvent(shop_data).into());
            }
            ServerShopMessages::ShopContents {
                shop_block,
                structure_entity,
                shop_data,
            } => {
                todo!();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        listen_netty
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );
}
