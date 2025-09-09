use std::fs;

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    entities::player::Player,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::{
        Item,
        usable::blueprint::{BlueprintItemData, DownloadBlueprint, DownloadBlueprintResponse, UploadBlueprint},
    },
    netty::{
        cosmos_encoder,
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    notifications::{Notification, NotificationKind},
    prelude::{Ship, Station, Structure},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::blueprint::{Blueprint, BlueprintAuthor, BlueprintType},
};
use uuid::Uuid;

use crate::{
    items::usable::UseHeldItemEvent,
    persistence::{
        loading::NeedsBlueprintLoaded,
        make_persistent::{DefaultPersistentComponent, make_persistent},
        saving::{BlueprintingSystemSet, NeedsBlueprinted, save_blueprint},
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
            name: "Blueprint".into(),
        };

        commands.entity(block.structure()).insert(NeedsBlueprinted {
            blueprint_type: Some(bp_data.blueprint_type),
            blueprint_name: format!("{id}"),
            name: "Blueprint".into(),
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

fn on_download_bp(
    mut nevr_download_bp: EventReader<NettyEventReceived<DownloadBlueprint>>,
    mut nevw_blueprint_response: NettyEventWriter<DownloadBlueprintResponse>,
) {
    for ev in nevr_download_bp.read() {
        let path = ev.blueprint_type.path_for(&ev.blueprint_id.to_string());

        match fs::read(&path) {
            Ok(data) => {
                let Ok(blueprint) = cosmos_encoder::deserialize::<Blueprint>(&data) else {
                    error!("Error deserializing blueprint @ {path:?}");
                    continue;
                };

                nevw_blueprint_response.write(
                    DownloadBlueprintResponse {
                        blueprint,
                        blueprint_id: ev.blueprint_id,
                    },
                    ev.client_id,
                );
            }
            Err(e) => {
                error!("Error sending blueprint {ev:?} - {e:?}");
            }
        }
    }
}

fn on_upload_blueprint(
    lobby: Res<ServerLobby>,
    mut q_player: Query<(&Player, &mut Inventory)>,
    mut nevr_upload_blueprint: EventReader<NettyEventReceived<UploadBlueprint>>,
    q_bp_data: Query<(), With<BlueprintItemData>>,
    mut commands: Commands,
    items: Res<Registry<Item>>,
    mut nevw_notif: NettyEventWriter<Notification>,
) {
    for ev in nevr_upload_blueprint.read() {
        let Some((player, mut inv)) = lobby.player_from_id(ev.client_id).and_then(|e| q_player.get_mut(e).ok()) else {
            continue;
        };

        let Some(blueprint) = items.from_id("cosmos:blueprint") else {
            continue;
        };

        if inv
            .itemstack_at(ev.slot as usize)
            .map(|x| x.item_id() == blueprint.id())
            .unwrap_or(false)
        {
            warn!("Player not holding blueprint at that slot!");
            continue;
        }

        if inv.query_itemstack_data(ev.slot as usize, &q_bp_data).is_some() {
            warn!("This blueprint already has data!");
            continue;
        }

        let mut blueprint = ev.blueprint.clone();
        blueprint.set_author(BlueprintAuthor::Player {
            name: player.name().to_owned(),
            id: player.client_id(),
        });

        let id = Uuid::new_v4();

        if let Err(e) = save_blueprint(&ev.blueprint, &id.to_string()) {
            error!("Error saving blueprint! {e:?}");

            nevw_notif.write(
                Notification::new(format!("Error Uploading Blueprint"), NotificationKind::Error),
                ev.client_id,
            );
            continue;
        }

        inv.insert_itemstack_data(
            ev.slot as usize,
            BlueprintItemData {
                blueprint_id: id,
                blueprint_type: blueprint.kind(),
                name: blueprint.name().to_owned(),
            },
            &mut commands,
        );

        nevw_notif.write(
            Notification::new(format!("Successfully Uploaded {}", blueprint.name()), NotificationKind::Info),
            ev.client_id,
        );
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<BlueprintItemData>(app);

    app.add_systems(OnEnter(GameState::PostLoading), register_blueprint_item)
        .add_systems(
            FixedUpdate,
            (on_use_blueprint, on_download_bp, on_upload_blueprint)
                .before(BlueprintingSystemSet::BeginBlueprinting)
                .in_set(NetworkingSystemsSet::Between),
        )
        .register_type::<BlueprintItemData>();
}
