use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    inventory::{Inventory, itemstack::ItemStackData},
    item::{
        Item,
        usable::{
            UseHeldItemEvent,
            blueprint::{BlueprintItemData, DownloadBlueprint, DownloadBlueprintResponse},
        },
    },
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    registry::{Registry, identifiable::Identifiable},
};

use crate::{
    create_private_button_event,
    ui::{
        OpenMenu,
        components::{
            button::{CosmosButton, register_button},
            window::GuiWindow,
        },
        font::DefaultFont,
    },
};

#[derive(Component)]
struct OpenedBp(BlueprintItemData);

fn on_use_blueprint(
    items: Res<Registry<Item>>,
    mut evr_use_item: EventReader<UseHeldItemEvent>,
    q_inventory: Query<&Inventory, With<LocalPlayer>>,
    q_blueprint_data: Query<&BlueprintItemData>,
    mut commands: Commands,
    font: Res<DefaultFont>,
) {
    for ev in evr_use_item.read() {
        let Some(id) = ev.item else {
            continue;
        };

        let Some(bp_item) = items.from_id("cosmos:blueprint") else {
            continue;
        };

        if bp_item.id() != id {
            continue;
        };

        let Ok(inv) = q_inventory.get(ev.player) else {
            continue;
        };

        if inv.itemstack_at(ev.held_slot).map(|x| x.item_id() != id).unwrap_or(false) {
            continue;
        }

        let Some(data) = inv.query_itemstack_data(ev.held_slot, &q_blueprint_data) else {
            continue;
        };

        commands
            .spawn((
                Name::new("Blueprint Window"),
                GuiWindow {
                    title: "Blueprint".into(),
                    body_styles: Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Node {
                    margin: UiRect::all(Val::Auto),
                    position_type: PositionType::Absolute,
                    width: Val::Px(800.0),
                    height: Val::Px(800.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                OpenMenu::new(0),
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new("Blueprint"),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(css::AQUA.into()),
                ));

                p.spawn((
                    Text::new(data.name.clone()),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..default()
                    },
                ));

                p.spawn(Node {
                    flex_grow: 1.0,
                    ..Default::default()
                });

                p.spawn((
                    OpenedBp(data.clone()),
                    CosmosButton::<SaveBlueprint> {
                        text: Some((
                            "Export".into(),
                            TextFont {
                                font: font.get(),
                                font_size: 24.0,
                                ..Default::default()
                            },
                            Default::default(),
                        )),
                        ..Default::default()
                    },
                    Node {
                        padding: UiRect::all(Val::Px(8.0)),
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    BackgroundColor(css::LIGHT_GREY.into()),
                ));
            });
        break;
    }
}

create_private_button_event!(SaveBlueprint);

fn on_export(
    mut evr_save: EventReader<SaveBlueprint>,
    mut nevw_download_bp: NettyEventWriter<DownloadBlueprint>,
    q_item_data: Query<&OpenedBp>,
) {
    for ev in evr_save.read() {
        let Ok(blueprint_data) = q_item_data.get(ev.0) else {
            continue;
        };

        nevw_download_bp.write(DownloadBlueprint {
            blueprint_id: blueprint_data.0.blueprint_id,
            blueprint_type: blueprint_data.0.blueprint_type,
        });
    }
}

fn on_receive_download(mut nevr_download: EventReader<DownloadBlueprintResponse>) {
    for ev in nevr_download.read() {
        info!("{:?}", ev.data);
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<SaveBlueprint>(app);

    app.add_systems(FixedUpdate, on_use_blueprint.in_set(FixedUpdateSet::Main))
        .add_systems(Update, (on_export, on_receive_download));
}
