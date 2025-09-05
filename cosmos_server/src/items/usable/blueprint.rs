use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    entities::player::Player,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::{
        Item,
        usable::blueprint::{BlueprintItemData, BlueprintType},
    },
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    notifications::{Notification, NotificationKind},
    prelude::{Ship, Station, Structure},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use uuid::Uuid;

use crate::{
    items::usable::UseHeldItemEvent,
    persistence::{
        make_persistent::{DefaultPersistentComponent, make_persistent},
        saving::{BlueprintingSystemSet, NeedsBlueprinted},
    },
};

impl DefaultPersistentComponent for BlueprintItemData {}

fn on_use_blueprint(
    mut q_player: Query<(&Player, &mut Inventory)>,
    mut evr_use_item: EventReader<UseHeldItemEvent>,
    q_structure: Query<(&Structure, Has<Station>, Has<Ship>)>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    mut nevw_notification: NettyEventWriter<Notification>,
    q_blueprint_data: Query<(), With<BlueprintItemData>>,
    mut commands: Commands,
) {
    for ev in evr_use_item.read() {
        let Ok((player, mut inv)) = q_player.get_mut(ev.player) else {
            continue;
        };
        let Some(blueprint_item) = items.from_id("cosmos:blueprint") else {
            return;
        };

        if ev.item != Some(blueprint_item.id()) {
            continue;
        }

        let Some(block) = ev.looking_at_block else {
            continue;
        };

        if inv.query_itemstack_data(ev.held_slot, &q_blueprint_data).is_some() {
            nevw_notification.write(
                Notification::new("This already contains a blueprint.", NotificationKind::Error),
                player.client_id(),
            );
            continue;
        }

        let Ok((structure, station, ship)) = q_structure.get(block.structure()) else {
            nevw_notification.write(
                Notification::new("Blueprint can only be used on ships and stations.", NotificationKind::Error),
                player.client_id(),
            );
            continue;
        };

        let block_name = structure.block_at(block.coords(), &blocks).unlocalized_name();

        if !((block_name == "cosmos:station_core" && station) || (block_name == "cosmos:ship_core" && ship)) {
            nevw_notification.write(
                Notification::new("Blueprint can only be used on the structure's core block.", NotificationKind::Error),
                player.client_id(),
            );
            continue;
        }

        let id = Uuid::new_v4();

        let bp_data = BlueprintItemData {
            blueprint_id: id,
            blueprint_type: if ship { BlueprintType::Ship } else { BlueprintType::Station },
            name: "Cool Blueprint".into(),
        };

        commands.entity(block.structure()).insert(NeedsBlueprinted {
            subdir_name: bp_data.blueprint_type.blueprint_directory().to_owned(),
            blueprint_name: format!("{id}"),
        });

        inv.insert_itemstack_data(ev.held_slot, bp_data, &mut commands);

        nevw_notification.write(Notification::new("Blueprint Created", NotificationKind::Info), player.client_id());
    }
}

fn register_blueprint_item(items: Res<Registry<Item>>, mut needs_data: ResMut<ItemShouldHaveData>) {
    if let Some(blueprint_item) = items.from_id("cosmos:blueprint") {
        needs_data.add_item(blueprint_item);
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<BlueprintItemData>(app);

    app.add_systems(OnEnter(GameState::PostLoading), register_blueprint_item)
        .add_systems(
            FixedUpdate,
            on_use_blueprint
                .before(BlueprintingSystemSet::BeginBlueprinting)
                .in_set(NetworkingSystemsSet::Between),
        )
        .register_type::<BlueprintItemData>();
}
