use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::mut_events::MutMessage,
    netty::{NettyChannelServer, cosmos_encoder, system_sets::NetworkingSystemsSet},
    shop::netty::ServerShopMessages,
    state::GameState,
    structure::structure_block::StructureBlock,
};

use super::{PurchasedMessage, SoldMessage, ui::OpenShopUiMessage};

fn shop_listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer_open_shop_ui: MessageWriter<MutMessage<OpenShopUiMessage>>,
    mut ev_writer_purchased: MessageWriter<PurchasedMessage>,
    mut ev_writer_sold: MessageWriter<SoldMessage>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Shop) {
        let msg: ServerShopMessages = cosmos_encoder::deserialize(&message).expect("Bad shop message");

        match msg {
            ServerShopMessages::OpenShop {
                shop_block,
                structure_entity,
                shop_data,
            } => {
                ev_writer_open_shop_ui.write(
                    OpenShopUiMessage {
                        shop: shop_data,
                        structure_block: StructureBlock::new(shop_block, structure_entity),
                    }
                    .into(),
                );
            }
            ServerShopMessages::PurchaseResult {
                shop_block,
                structure_entity,
                details,
            } => {
                ev_writer_purchased.write(PurchasedMessage {
                    details,
                    shop_block,
                    structure_entity,
                });
            }
            ServerShopMessages::SellResult {
                shop_block,
                structure_entity,
                details,
            } => {
                ev_writer_sold.write(SoldMessage {
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
        shop_listen_netty
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );
}
