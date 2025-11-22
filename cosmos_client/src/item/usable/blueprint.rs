use std::fs;

use bevy::{
    color::palettes::css,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::player::creative::Creative,
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    item::{
        Item,
        usable::{
            UseHeldItemMessage,
            blueprint::{
                BlueprintItemData, ClearBlueprint, CopyBlueprint, DownloadBlueprint, DownloadBlueprintResponse, RequestLoadBlueprint,
                UploadBlueprint,
            },
        },
    },
    netty::{client::LocalPlayer, cosmos_encoder, sync::events::client_event::NettyMessageWriter},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::blueprint::Blueprint,
};
use futures_lite::future;
use rfd::AsyncFileDialog;

use crate::{
    interactions::block_interactions::LookingAt,
    ui::{
        OpenMenu,
        components::{
            button::{ButtonMessage, CosmosButton},
            window::GuiWindow,
        },
        font::DefaultFont,
    },
};

#[derive(Component)]
struct OpenedBp(BlueprintItemData);

fn on_use_blueprint(
    items: Res<Registry<Item>>,
    mut evr_use_item: MessageReader<UseHeldItemMessage>,
    q_player: Query<(&Inventory, &LookingAt, Has<Creative>), With<LocalPlayer>>,
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

        let Ok((inv, looking_at, creative)) = q_player.get(ev.player) else {
            continue;
        };

        if inv.itemstack_at(ev.held_slot).map(|x| x.item_id() != id).unwrap_or(false) {
            continue;
        }

        let Some(data) = inv.query_itemstack_data(ev.held_slot, &q_blueprint_data) else {
            if looking_at.looking_at_block.is_some() {
                // Server handles this
                continue;
            }

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
                        width: Val::Px(400.0),
                        height: Val::Px(400.0),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    OpenMenu::new(0),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("Right click a ship or station core to create a blueprint of it."),
                        TextFont {
                            font: font.get(),
                            font_size: 24.0,
                            ..Default::default()
                        },
                    ));

                    p.spawn(Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    });

                    p.spawn((
                        CosmosButton {
                            text: Some((
                                "Load".into(),
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
                    ))
                    .observe(on_load);
                });
            break;
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
                    width: Val::Px(400.0),
                    height: Val::Px(400.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..Default::default()
                },
                OpenMenu::new(0),
            ))
            .with_children(|p| {
                p.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    ..Default::default()
                })
                .with_children(|p| {
                    p.spawn((
                        CosmosButton {
                            text: Some((
                                "Clear".into(),
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
                            width: Val::Percent(40.0),
                            height: Val::Px(64.0),
                            ..Default::default()
                        },
                        BackgroundColor(css::RED.into()),
                    ))
                    .observe(on_clear);

                    p.spawn((
                        CosmosButton {
                            text: Some((
                                "Copy".into(),
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
                            width: Val::Percent(40.0),
                            height: Val::Px(64.0),
                            ..Default::default()
                        },
                        BackgroundColor(css::GREEN.into()),
                    ))
                    .observe(on_copy);
                });

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

                if creative {
                    p.spawn((
                        CosmosButton {
                            text: Some((
                                "Load".into(),
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
                            width: Val::Percent(100.0),
                            margin: UiRect::bottom(Val::Px(30.0)),
                            ..Default::default()
                        },
                        BackgroundColor(css::GREEN.into()),
                    ))
                    .observe(load_clicked);
                }

                p.spawn((
                    OpenedBp(data.clone()),
                    CosmosButton {
                        text: Some((
                            "Download".into(),
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
                ))
                .observe(on_export);
            });
        break;
    }
}

fn on_export(ev: On<ButtonMessage>, mut nevw_download_bp: NettyMessageWriter<DownloadBlueprint>, q_item_data: Query<&OpenedBp>) {
    let Ok(blueprint_data) = q_item_data.get(ev.0) else {
        return;
    };

    nevw_download_bp.write(DownloadBlueprint {
        blueprint_id: blueprint_data.0.blueprint_id,
        blueprint_type: blueprint_data.0.blueprint_type,
    });
}

#[derive(Resource)]
struct LoadTask(Task<Option<(u32, Vec<u8>)>>);

fn on_load(
    _trigger: On<ButtonMessage>,
    q_held_item: Query<&HeldItemSlot, With<LocalPlayer>>,
    mut commands: Commands,
    loading_already: Option<Res<LoadTask>>,
) {
    if loading_already.is_some() {
        return;
    }

    let Ok(held_item) = q_held_item.single() else {
        return;
    };

    let bp_slot = held_item.slot();

    let task = AsyncComputeTaskPool::get().spawn(async move {
        let _ = fs::create_dir("./blueprints");
        let cur_dir = std::env::current_dir().unwrap_or_default();
        let file = AsyncFileDialog::new()
            .add_filter("Blueprints", &["bp"])
            .set_directory(cur_dir.join("./blueprints/"))
            .set_title("Save Blueprint")
            .set_can_create_directories(true)
            .pick_file()
            .await;

        let handle = file?;
        fs::read(handle.path()).ok().map(|data| (bp_slot, data))
    });

    commands.insert_resource(LoadTask(task));

    // nevw_upload_bp.write(UploadBlueprint{
    //     name: "Blueprint".into(),
    //     data:
    //     blueprint_id: blueprint_data.0.blueprint_id,
    //     blueprint_type: blueprint_data.0.blueprint_type,
    // });
}

fn upload_selected_blueprint(
    mut commands: Commands,
    mut load_task: ResMut<LoadTask>,
    mut nevw_upload_bp: NettyMessageWriter<UploadBlueprint>,
) {
    let Some(data) = future::block_on(future::poll_once(&mut load_task.0)) else {
        return;
    };

    commands.remove_resource::<LoadTask>();

    let Some((slot, data)) = data else {
        return;
    };

    let Ok(blueprint) = cosmos_encoder::deserialize::<Blueprint>(&data) else {
        error!("Invalid blueprint data!");
        return;
    };

    nevw_upload_bp.write(UploadBlueprint { blueprint, slot });
}

fn on_receive_download(mut nevr_download: MessageReader<DownloadBlueprintResponse>) {
    for ev in nevr_download.read() {
        let thread_pool = AsyncComputeTaskPool::get();

        let data = cosmos_encoder::serialize(&ev.blueprint);

        let task = thread_pool.spawn(async move {
            let _ = fs::create_dir("./blueprints");
            let cur_dir = std::env::current_dir().unwrap_or_default();
            let file = AsyncFileDialog::new()
                .add_filter("Blueprints", &["bp"])
                .set_directory(cur_dir.join("./blueprints/"))
                .set_title("Save Blueprint")
                .set_file_name("blueprint.bp")
                .set_can_create_directories(true)
                .save_file()
                .await;

            if let Some(handle) = file {
                if let Err(e) = fs::write(handle.path(), data) {
                    error!("Error saving blueprint - {e:?}");
                } else {
                    info!("Successfully saved blueprint to {}", handle.path().to_string_lossy());
                }
            }
        });

        task.detach();
    }
}

fn on_clear(
    _trigger: On<ButtonMessage>,
    mut nevw_clear: NettyMessageWriter<ClearBlueprint>,
    q_held_item: Query<&HeldItemSlot, With<LocalPlayer>>,
) {
    let Ok(held_item) = q_held_item.single() else {
        return;
    };

    nevw_clear.write(ClearBlueprint { slot: held_item.slot() });
}

fn on_copy(
    _trigger: On<ButtonMessage>,
    mut nevw_copy: NettyMessageWriter<CopyBlueprint>,
    q_held_item: Query<&HeldItemSlot, With<LocalPlayer>>,
) {
    let Ok(held_item) = q_held_item.single() else {
        return;
    };

    nevw_copy.write(CopyBlueprint { slot: held_item.slot() });
}

fn load_clicked(
    _trigger: On<ButtonMessage>,
    mut nevw_load_bp: NettyMessageWriter<RequestLoadBlueprint>,
    q_held_item: Query<&HeldItemSlot, With<LocalPlayer>>,
) {
    let Ok(held_item) = q_held_item.single() else {
        return;
    };

    nevw_load_bp.write(RequestLoadBlueprint { slot: held_item.slot() });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_use_blueprint.in_set(FixedUpdateSet::Main))
        .add_systems(
            Update,
            (on_receive_download, upload_selected_blueprint.run_if(resource_exists::<LoadTask>)).run_if(in_state(GameState::Playing)),
        );
}
