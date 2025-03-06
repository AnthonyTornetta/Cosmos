use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::{
        blocks::COLOR_VALUES,
        specific_blocks::dye_machine::{DyeBlock, OpenDyeMachine},
    },
    netty::{
        client::LocalPlayer,
        sync::{
            events::client_event::NettyEventWriter,
            mapping::{Mappable, NetworkMapping},
        },
        system_sets::NetworkingSystemsSet,
    },
    prelude::{Structure, StructureBlock},
    state::GameState,
};

use crate::{
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed, InventorySide},
    ui::{
        components::{
            button::{register_button, ButtonEvent, ButtonStyles, CosmosButton},
            window::GuiWindow,
        },
        font::DefaultFont,
        OpenMenu, UiSystemSet,
    },
};

#[derive(Component)]
struct OpenDyeUi;

fn open_dye_ui(
    font: Res<DefaultFont>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
    mut commands: Commands,
    mut evr_open_ui: EventReader<OpenDyeMachine>,
    q_structure: Query<&Structure>,
) {
    let Some(ev) = evr_open_ui.read().next() else {
        return;
    };

    let Ok(structure) = q_structure.get(ev.0.structure()) else {
        return;
    };

    let Ok(lp) = q_local_player.get_single() else {
        return;
    };

    commands.entity(lp).insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));

    let mut slot_entity = None;

    commands
        .spawn((
            OpenDyeUi,
            OpenMenu::new(0),
            Name::new("Dye Machine Ui"),
            BorderColor(Color::BLACK),
            GuiWindow {
                title: "Dye Machine".into(),
                body_styles: Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            },
            Node {
                right: Val::Px(200.0),
                left: Val::Auto,
                top: Val::Px(100.0),
                width: Val::Px(600.0),
                height: Val::Px(300.0),
                position_type: PositionType::Absolute,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                Node {
                    flex_grow: 1.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                slot_entity = Some(
                    p.spawn((
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..Default::default()
                        },
                        BackgroundColor(css::GRAY.into()),
                        Name::new("Inventory Slot"),
                    ))
                    .id(),
                );

                p.spawn((
                    Name::new("Color Buttons"),
                    Node {
                        margin: UiRect::top(Val::Px(32.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    for c in COLOR_VALUES {
                        p.spawn((
                            BtnColor(c, ev.0),
                            Node {
                                width: Val::Px(32.0),
                                height: Val::Px(32.0),
                                ..Default::default()
                            },
                            CosmosButton::<ColorBtnClicked> {
                                button_styles: Some(ButtonStyles {
                                    background_color: c.into(),
                                    hover_background_color: c.into(),
                                    press_background_color: c.into(),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ));
                    }
                });
            });
        });

    if let Some(ent) = structure.block_data(ev.0.coords()) {
        commands
            .entity(ent)
            .insert(InventoryNeedsDisplayed::Custom(CustomInventoryRender::new(vec![(
                0,
                slot_entity.expect("Set above"),
            )])));
    }
}

fn click_color_btn(
    netty_mapping: Res<NetworkMapping>,
    q_btn_color: Query<&BtnColor>,
    mut evr_color_btn: EventReader<ColorBtnClicked>,
    mut nevw_dye_block: NettyEventWriter<DyeBlock>,
) {
    for ev in evr_color_btn.read() {
        let Ok(btn_color) = q_btn_color.get(ev.0) else {
            error!("No button color componnet!");
            continue;
        };

        if let Some(b) = btn_color.1.map_to_server(&netty_mapping).ok() {
            nevw_dye_block.send(DyeBlock {
                block: b,
                color: btn_color.0,
            });
        }
    }
}

#[derive(Component, Reflect, Debug)]
struct BtnColor(Srgba, StructureBlock);

#[derive(Event, Debug)]
struct ColorBtnClicked(Entity);

impl ButtonEvent for ColorBtnClicked {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<ColorBtnClicked>(app);

    app.add_systems(
        Update,
        (
            open_dye_ui.in_set(UiSystemSet::PreDoUi),
            click_color_btn.in_set(UiSystemSet::FinishUi),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<BtnColor>();
}
