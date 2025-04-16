use bevy::{
    app::{App, Update},
    ecs::{event::EventWriter, schedule::IntoSystemConfigs, system::ResMut},
    state::condition::in_state,
};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::mut_events::MutEvent,
    netty::{NettyChannelServer, cosmos_encoder, system_sets::NetworkingSystemsSet},
    shop::netty::ServerShopMessages,
    state::GameState,
    structure::structure_block::StructureBlock,
};

use super::{PurchasedEvent, SoldEvent, ui::OpenShopUiEvent};

fn shop_listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer_open_shop_ui: EventWriter<MutEvent<OpenShopUiEvent>>,
    mut ev_writer_purchased: EventWriter<PurchasedEvent>,
    mut ev_writer_sold: EventWriter<SoldEvent>,
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
                ev_writer_purchased.send(PurchasedEvent {
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
                ev_writer_sold.send(SoldEvent {
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
