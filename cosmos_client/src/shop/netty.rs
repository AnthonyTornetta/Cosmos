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
    structure::structure_block::StructureBlock,
};

use crate::state::game_state::GameState;

use super::{ui::OpenShopUiEvent, PurchasedEvent};

fn listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer_open_shop_ui: EventWriter<MutEvent<OpenShopUiEvent>>,
    mut ev_writer_purchased: EventWriter<PurchasedEvent>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Shop) {
        let msg: ServerShopMessages = cosmos_encoder::deserialize(&message).expect("Bad shop message");

        match msg {
            ServerShopMessages::OpenShop {
                shop_block,
                structure_entity,
                shop_data,
            } => {
                ev_writer_open_shop_ui.send(
                    OpenShopUiEvent {
                        shop: shop_data,
                        structure_block: StructureBlock::new(shop_block),
                        structure_entity,
                    }
                    .into(),
                );
            }
            ServerShopMessages::ShopContents {
                shop_block,
                structure_entity,
                shop_data,
            } => {
                todo!();
            }
            ServerShopMessages::Purchase {
                shop_block,
                structure_entity,
                details,
            } => {
                ev_writer_purchased.send(PurchasedEvent {
                    details,
                    shop_block,
                    structure_entity,
                });
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
