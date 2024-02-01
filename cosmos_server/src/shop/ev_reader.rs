use bevy::{
    app::{App, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut},
    },
    log::info,
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    entities::player::Player,
    item::Item,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    shop::{netty::ServerShopMessages, Shop, ShopEntry},
    structure::Structure,
};

fn on_interact_with_shop(
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
    q_player: Query<&Player>,
    blocks: Res<Registry<Block>>,
    mut ev_reader: EventReader<BlockInteractEvent>,
    items: Res<Registry<Item>>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = ev.structure_block.block(structure, &blocks);

        if block.unlocalized_name() == "cosmos:shop" {
            let fake_shop_data = Shop {
                name: "Fake Shop".into(),
                contents: vec![
                    ShopEntry::Selling {
                        item_id: items.from_id("cosmos:laser_cannon").expect("Missing laser_cannon").id(),
                        max_quantity_selling: 1000,
                        price_per: 100,
                    },
                    ShopEntry::Selling {
                        item_id: items.from_id("cosmos:plasma_drill").expect("Missing plasma_drill").id(),
                        max_quantity_selling: 1000,
                        price_per: 100,
                    },
                    ShopEntry::Selling {
                        item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing ship_hull_grey").id(),
                        max_quantity_selling: 1000,
                        price_per: 100,
                    },
                    ShopEntry::Selling {
                        item_id: items.from_id("cosmos:thruster").expect("Missing thruster").id(),
                        max_quantity_selling: 1000,
                        price_per: 100,
                    },
                    ShopEntry::Buying {
                        item_id: items.from_id("cosmos:laser_cannon").expect("Missing laser_cannon").id(),
                        max_quantity_buying: Some(2),
                        price_per: 100,
                    },
                    ShopEntry::Buying {
                        item_id: items.from_id("cosmos:plasma_drill").expect("Missing plasma_drill").id(),
                        max_quantity_buying: Some(10000),
                        price_per: 100,
                    },
                    ShopEntry::Buying {
                        item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing ship_hull_grey").id(),
                        max_quantity_buying: None,
                        price_per: 100,
                    },
                    ShopEntry::Buying {
                        item_id: items.from_id("cosmos:thruster").expect("Missing thruster").id(),
                        max_quantity_buying: None,
                        price_per: 100,
                    },
                ],
            };

            server.send_message(
                player.id(),
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::OpenShop {
                    shop_block: ev.structure_block.coords(),
                    structure_entity: ev.structure_entity,
                    shop_data: fake_shop_data,
                }),
            );
            info!("Interacted w/ shop");
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_interact_with_shop.after(NetworkingSystemsSet::FlushReceiveMessages));
}
