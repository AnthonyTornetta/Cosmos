use bevy::{
    app::{App, Update},
    asset::AssetServer,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, With},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::error,
    render::color::Color,
    text::{Text, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        widget::Label,
        BackgroundColor, BorderColor, FlexDirection, JustifyContent, Style, UiRect, Val,
    },
};
use cosmos_core::{
    ecs::{
        mut_events::{MutEvent, MutEventsCommand},
        NeedsDespawned,
    },
    item::Item,
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    shop::{Shop, ShopEntry},
};

use crate::{
    lang::Lang,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles, ButtonUiSystemSet},
            scollable_container::{ScrollBox, ScrollBundle},
            slider::{Slider, SliderBundle},
            text_input::{InputType, TextInput, TextInputBundle},
            window::{GuiWindow, WindowBundle},
        },
        UiSystemSet,
    },
};

#[derive(Event)]
pub(super) struct OpenShopUiEvent(pub Shop);

#[derive(Component)]
struct ShopUi {
    shop: Shop,
    selected_item: Option<SelectedItem>,
}

#[derive(Component)]
struct ShopUiEntity(Entity);

struct DescriptionSection(Entity);
struct PriceSection(Entity);

struct SelectedItem {
    entry: ShopEntry,
    amount_selected: u32,
}

fn open_shop_ui(mut commands: Commands, mut ev_reader: EventReader<MutEvent<OpenShopUiEvent>>, q_open_shops: Query<Entity, With<ShopUi>>) {
    for ev in ev_reader.read() {
        let shop = std::mem::take(&mut ev.write().0);

        println!("Display: {shop:?}");

        for ent in q_open_shops.iter() {
            commands.entity(ent).insert(NeedsDespawned);
        }

        commands.spawn(ShopUi { shop, selected_item: None });
    }
}

fn render_shop_ui(
    mut commands: Commands,
    q_shop_ui: Query<(&ShopUi, Entity), Added<ShopUi>>,
    asset_server: Res<AssetServer>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
) {
    let Ok((shop_ui, ui_ent)) = q_shop_ui.get_single() else {
        return;
    };

    let name = &shop_ui.shop.name;

    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let text_style_small = TextStyle {
        color: Color::WHITE,
        font_size: 24.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    commands
        .entity(ui_ent)
        .insert(WindowBundle {
            node_bundle: NodeBundle {
                background_color: Color::hex("2D2D2D").unwrap().into(),
                style: Style {
                    width: Val::Px(1000.0),
                    height: Val::Px(800.0),
                    left: Val::Percent(51.0),
                    margin: UiRect {
                        // Centers it vertically
                        top: Val::Auto,
                        bottom: Val::Auto,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            window: GuiWindow {
                title: name.into(),
                body_styles: Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|p| {
            p.spawn(NodeBundle {
                style: Style {
                    height: Val::Px(50.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|p| {
                p.spawn(ButtonBundle::<ClickSellTabEvent> {
                    node_bundle: NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    button: Button {
                        text: Some(("Sell".into(), text_style.clone())),
                        ..Default::default()
                    },
                    ..Default::default()
                });

                p.spawn(ButtonBundle::<ClickBuyTabEvent> {
                    node_bundle: NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    button: Button {
                        button_styles: Some(ButtonStyles {
                            background_color: Color::GREEN,
                            hover_background_color: Color::DARK_GREEN,
                            press_background_color: Color::DARK_GREEN,
                            ..Default::default()
                        }),
                        text: Some(("Buy".into(), text_style.clone())),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });

            p.spawn((
                Name::new("Body"),
                NodeBundle {
                    border_color: Color::hex("1C1C1C").unwrap().into(),
                    style: Style {
                        border: UiRect {
                            bottom: Val::Px(4.0),
                            top: Val::Px(4.0),
                            ..Default::default()
                        },
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Main Stuff"),
                    NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            padding: UiRect {
                                left: Val::Px(40.0),
                                right: Val::Px(40.0),
                                top: Val::Px(20.0),
                                bottom: Val::Px(20.0),
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|body| {
                    body.spawn((
                        Name::new("Description section"),
                        NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn(TextBundle {
                            text: Text::from_section("Laser Cannon", text_style.clone()),
                            style: Style {
                                margin: UiRect {
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        });

                        p.spawn(TextBundle {
                            text: Text::from_section("Description of Cannon", text_style_small.clone()),
                            style: Style {
                                margin: UiRect {
                                    bottom: Val::Px(30.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        });

                        p.spawn(TextBundle {
                            text: Text::from_section("Stats", text_style.clone()),
                            style: Style {
                                margin: UiRect {
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                    });

                    body.spawn((Name::new("Item picture"), NodeBundle { ..Default::default() }));
                });

                body.spawn((
                    Name::new("Shop Categories"),
                    NodeBundle {
                        border_color: Color::hex("1C1C1C").unwrap().into(),
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            width: Val::Px(400.0),
                            border: UiRect {
                                left: Val::Px(4.0),
                                ..Default::default()
                            },
                            padding: UiRect::all(Val::Px(10.0)),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|body| {
                    body.spawn((
                        Name::new("Stock Header Text"),
                        Label,
                        TextBundle {
                            text: Text::from_section("Stock", text_style.clone()),
                            style: Style {
                                margin: UiRect {
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ));

                    body.spawn((
                        Name::new("Search Text Box"),
                        TextInputBundle {
                            text_input: TextInput {
                                style: text_style.clone(),
                                ..Default::default()
                            },
                            node_bundle: NodeBundle {
                                border_color: Color::hex("111111").unwrap().into(),
                                background_color: Color::hex("555555").unwrap().into(),
                                style: Style {
                                    border: UiRect::all(Val::Px(2.0)),
                                    padding: UiRect {
                                        top: Val::Px(4.0),
                                        bottom: Val::Px(4.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ));

                    body.spawn((
                        Name::new("Items List"),
                        ScrollBundle {
                            node_bundle: NodeBundle {
                                style: Style {
                                    flex_grow: 1.0,
                                    margin: UiRect::top(Val::Px(10.0)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            slider: ScrollBox { ..Default::default() },
                            ..Default::default()
                        },
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Name::new("Contents"),
                            NodeBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(10.0)),
                                    flex_direction: FlexDirection::Column,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .with_children(|p| {
                            for shop_entry in &shop_ui.shop.contents {
                                let ShopEntry::Selling {
                                    item_id,
                                    max_quantity_selling,
                                    price_per,
                                } = shop_entry
                                else {
                                    continue;
                                };

                                let item = items.from_numeric_id(*item_id);
                                let display_name = lang.get_name(item).unwrap_or(item.unlocalized_name());

                                p.spawn((
                                    Name::new(display_name.to_owned()),
                                    *shop_entry,
                                    ShopUiEntity(ui_ent),
                                    ButtonBundle::<ClickItemEvent> {
                                        button: Button {
                                            // text: Some((display_name.to_owned(), text_style_small.clone())),
                                            ..Default::default()
                                        },
                                        node_bundle: NodeBundle {
                                            style: Style {
                                                flex_direction: FlexDirection::Row,
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                ))
                                .with_children(|p| {
                                    p.spawn((
                                        Name::new("Item Name"),
                                        TextBundle {
                                            text: Text::from_section(display_name, text_style_small.clone()),
                                            style: Style {
                                                flex_grow: 1.0,
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                    ));
                                    p.spawn((
                                        Name::new("Quantity"),
                                        TextBundle {
                                            text: Text::from_section(format!("({max_quantity_selling})"), text_style_small.clone()),
                                            ..Default::default()
                                        },
                                    ));
                                });
                            }
                        });

                        // for i in 0..100 {
                        // p.spawn((TextBundle::from_section(format!("Item {i}"), text_style_small.clone()), Label));
                        // }
                    });
                });
            });

            p.spawn((
                Name::new("Footer"),
                NodeBundle {
                    style: Style {
                        padding: UiRect::top(Val::Px(10.0)),
                        // height: Val::Px(170.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn(NodeBundle {
                    style: Style {
                        flex_grow: 2.0,
                        flex_direction: FlexDirection::Column,
                        padding: UiRect {
                            bottom: Val::Px(10.0),
                            top: Val::Px(0.0),
                            left: Val::Px(20.0),
                            right: Val::Px(20.0),
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|p| {
                    p.spawn(NodeBundle {
                        style: Style {
                            padding: UiRect {
                                left: Val::Px(20.0),
                                bottom: Val::Px(10.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn(TextBundle {
                            text: Text::from_section("$1000", text_style.clone()),
                            ..Default::default()
                        });
                    });

                    p.spawn(TextBundle {
                        text: Text::from_section("- $20 x 6 = $120", text_style.clone()),
                        style: Style {
                            bottom: Val::Px(10.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    });

                    p.spawn(NodeBundle {
                        border_color: Color::hex("555555").unwrap().into(),
                        style: Style {
                            border: UiRect {
                                top: Val::Px(5.0),
                                ..Default::default()
                            },
                            padding: UiRect {
                                top: Val::Px(10.0),
                                left: Val::Px(20.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn(TextBundle {
                            text: Text::from_section("$880", text_style.clone()),
                            ..Default::default()
                        });
                    });
                });

                p.spawn(NodeBundle {
                    style: Style {
                        flex_grow: 3.0,
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|p| {
                    p.spawn(NodeBundle {
                        style: Style { ..Default::default() },
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn(TextInputBundle {
                            node_bundle: NodeBundle {
                                style: Style {
                                    width: Val::Px(250.0),
                                    padding: UiRect::all(Val::Px(10.0)),
                                    ..Default::default()
                                },
                                border_color: Color::hex("111111").unwrap().into(),
                                background_color: Color::hex("555555").unwrap().into(),
                                ..Default::default()
                            },
                            text_input: TextInput {
                                input_type: InputType::Integer { min: 0, max: 1000 },
                                style: text_style.clone(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });

                        p.spawn(NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                margin: UiRect {
                                    right: Val::Px(10.0),
                                    left: Val::Px(20.0),
                                    ..Default::default()
                                },
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::SpaceBetween,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|p| {
                            p.spawn(NodeBundle {
                                style: Style {
                                    flex_grow: 1.0,
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|p| {
                                p.spawn(TextBundle {
                                    text: Text::from_section("0", text_style_small.clone()),
                                    ..Default::default()
                                });

                                p.spawn(TextBundle {
                                    text: Text::from_section("1000", text_style_small.clone()),
                                    ..Default::default()
                                });
                            });

                            p.spawn(SliderBundle {
                                node_bundle: NodeBundle {
                                    style: Style { ..Default::default() },
                                    ..Default::default()
                                },
                                slider: Slider {
                                    range: 0..1001,
                                    background_color: Color::hex("999999").unwrap(),
                                    foreground_color: Color::AQUAMARINE,
                                    square_color: Color::hex("555555").unwrap(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });
                        });
                    });

                    p.spawn(ButtonBundle::<BuyBtnEvent> {
                        node_bundle: NodeBundle {
                            style: Style {
                                margin: UiRect::top(Val::Px(10.0)),
                                height: Val::Px(80.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        button: Button {
                            text: Some(("BUY".into(), text_style.clone())),
                            button_styles: Some(ButtonStyles {
                                background_color: Color::GREEN,
                                hover_background_color: Color::DARK_GREEN,
                                press_background_color: Color::DARK_GREEN,
                                ..Default::default()
                            }),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
            });

            // p.spawn(TextInputBundle {
            //     node_bundle: NodeBundle {
            //         background_color: Color::DARK_GRAY.into(),
            //         style: Style {
            //             width: Val::Px(200.0),
            //             height: Val::Px(40.0),
            //             ..Default::default()
            //         },
            //         ..Default::default()
            //     },
            //     text_input: TextInput {
            //         style: TextStyle {
            //             font_size: 32.0,
            //             ..Default::default()
            //         },
            //         input_type: InputType::Integer { min: -10000, max: 10000 },
            //         ..Default::default()
            //     },
            //     ..Default::default()
            // });

            // p.spawn(ButtonBundle::<ClickBtnEvent> {
            //     node_bundle: NodeBundle {
            //         style: Style {
            //             width: Val::Px(400.0),
            //             height: Val::Px(200.0),
            //             ..Default::default()
            //         },
            //         ..Default::default()
            //     },
            //     button: Button::<ClickBtnEvent> {
            //         text: Some(("Hello!".into(), text_style.clone())),
            //         button_styles: Some(ButtonStyles {
            //             background_color: Color::RED,
            //             foreground_color: Color::BLACK,
            //             hover_background_color: Color::GREEN,
            //             hover_foreground_color: Color::WHITE,
            //             press_background_color: Color::PURPLE,
            //             press_foreground_color: Color::YELLOW,
            //         }),
            //         ..Default::default()
            //     },

            //     ..Default::default()
            // });

            // p.spawn(SliderBundle {
            //     node_bundle: NodeBundle {
            //         style: Style {
            //             width: Val::Px(400.0),
            //             height: Val::Px(200.0),
            //             ..Default::default()
            //         },
            //         ..Default::default()
            //     },
            //     slider: Slider {
            //         range: 0..1001,
            //         slider_styles: Some(SliderStyles {
            //             hover_background_color: Color::GREEN,
            //             hover_foreground_color: Color::WHITE,
            //             press_background_color: Color::PURPLE,
            //             press_foreground_color: Color::YELLOW,
            //         }),
            //         ..Default::default()
            //     },
            //     ..Default::default()
            // });
        });
}

#[derive(Event, Debug)]
struct ClickSellTabEvent;

impl ButtonEvent for ClickSellTabEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct ClickBuyTabEvent;

impl ButtonEvent for ClickBuyTabEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Event, Debug)]
struct ClickBtnEvent {}

impl ButtonEvent for ClickBtnEvent {
    fn create_event(_: Entity) -> Self {
        Self {}
    }
}

#[derive(Event, Debug)]
struct BuyBtnEvent {}

impl ButtonEvent for BuyBtnEvent {
    fn create_event(_: Entity) -> Self {
        Self {}
    }
}

#[derive(Event, Debug)]
struct ClickItemEvent(Entity);

impl ButtonEvent for ClickItemEvent {
    fn create_event(entity: Entity) -> Self {
        Self(entity)
    }
}

fn reader(mut ev_reader: EventReader<ClickBtnEvent>) {
    for _ in ev_reader.read() {
        println!("Click event!");
    }
}

fn click_item_event(
    mut ev_reader: EventReader<ClickItemEvent>,
    q_shop_entry: Query<(&ShopEntry, &ShopUiEntity)>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
    mut q_shop: Query<&mut ShopUi>,
) {
    for ev in ev_reader.read() {
        let Ok((entry, shop_ui_ent)) = q_shop_entry.get(ev.0) else {
            error!("Shop item button didn't have shop entry or shop ui entity?");
            return;
        };

        let Ok(mut shop_ui) = q_shop.get_mut(shop_ui_ent.0) else {
            error!("Shop item button had invalid shop ui entity?");
            return;
        };

        if shop_ui.selected_item.as_ref().map(|x| x.entry != *entry).unwrap_or(true) {
            shop_ui.selected_item = Some(SelectedItem {
                entry: *entry,
                amount_selected: 0,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<ClickBtnEvent>(app);
    register_button::<ClickSellTabEvent>(app);
    register_button::<ClickBuyTabEvent>(app);
    register_button::<BuyBtnEvent>(app);
    register_button::<ClickItemEvent>(app);

    app.add_mut_event::<OpenShopUiEvent>()
        .add_systems(
            Update,
            (open_shop_ui, render_shop_ui)
                .after(NetworkingSystemsSet::FlushReceiveMessages)
                .before(UiSystemSet::ApplyDeferredA),
        )
        .add_systems(Update, reader.after(ButtonUiSystemSet::SendButtonEvents));
}
