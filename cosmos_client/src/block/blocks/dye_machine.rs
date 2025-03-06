use bevy::prelude::*;
use cosmos_core::{
    block::{blocks::COLOR_VALUES, specific_blocks::dye_machine::OpenDyeMachine},
    netty::system_sets::NetworkingSystemsSet,
};

use crate::{
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed},
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

fn open_dye_ui(font: Res<DefaultFont>, mut commands: Commands, mut evr_open_ui: EventReader<OpenDyeMachine>) {
    let Some(ev) = evr_open_ui.read().next() else {
        return;
    };

    let font_style = TextFont {
        font: font.0.clone_weak(),
        font_size: 24.0,
        ..Default::default()
    };

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
                height: Val::Px(200.0),
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
                        Name::new("Inventory Slot"),
                    ))
                    .id(),
                );

                p.spawn((
                    Name::new("Color Buttons"),
                    Node {
                        margin: UiRect::top(Val::Px(32.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceEvenly,
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    for c in COLOR_VALUES {
                        p.spawn((
                            BtnColor(c),
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

    commands
        .entity(ev.0.structure())
        .insert(InventoryNeedsDisplayed::Custom(CustomInventoryRender::new(vec![(
            0,
            slot_entity.expect("Set above"),
        )])));
}

fn click_color_btn(mut evr_color_btn: EventReader<ColorBtnClicked>) {
    for ev in evr_color_btn.read() {
        println!("GOT EVENT! {ev:?}");
    }
}

#[derive(Component, Reflect, Debug)]
struct BtnColor(Srgba);

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
            .in_set(NetworkingSystemsSet::Between),
    )
    .register_type::<BtnColor>();
}
