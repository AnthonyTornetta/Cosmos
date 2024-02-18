use bevy::{
    app::{App, Update},
    asset::AssetServer,
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, Changed, With},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::error,
    reflect::Reflect,
    render::color::Color,
    text::{Text, TextSection, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        widget::Label,
        Display, FlexDirection, JustifyContent, PositionType, Style, UiRect, Val,
    },
};
use cosmos_core::{
    economy::Credits,
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
    netty::flags::LocalPlayer,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::{ScrollBox, ScrollBundle},
            slider::{Slider, SliderBundle, SliderValue},
            text_input::{InputType, InputValue, TextInput, TextInputBundle},
            window::{GuiWindow, WindowBundle},
        },
        reactivity::{add_reactable_type, BindValue, BindValues, ReactableFields, ReactableValue},
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

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SelectedItemName(String);

impl ReactableValue for SelectedItemName {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SelectedItemDescription(String);

impl ReactableValue for SelectedItemDescription {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.to_owned();
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SelectedItemMaxQuantity(u32);

impl ReactableValue for SelectedItemMaxQuantity {
    fn as_value(&self) -> String {
        format!("{}", self.0)
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.parse().unwrap_or(0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct PricePerUnit(u32);

impl ReactableValue for PricePerUnit {
    fn as_value(&self) -> String {
        format!("{}", self.0)
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.parse().unwrap_or(0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct NetCredits(i64);

impl ReactableValue for NetCredits {
    fn as_value(&self) -> String {
        format!("{}", self.0)
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.parse().unwrap_or(0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct AmountSelected(u64);

impl ReactableValue for AmountSelected {
    fn as_value(&self) -> String {
        format!("{}", self.0)
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.parse().unwrap_or(0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Clone, Copy)]
enum ShopMode {
    Buy,
    Sell,
}

impl ReactableValue for ShopMode {
    fn as_value(&self) -> String {
        match *self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
        .into()
    }

    fn set_from_value(&mut self, new_value: &str) {
        match new_value {
            "BUY" => *self = Self::Buy,
            "SELL" => *self = Self::Sell,
            _ => {
                error!("Invalid buy/sell state: {new_value} (Valid types are 'BUY' or 'SELL'.");
                *self = Self::Buy;
            }
        }
    }
}

#[derive(Component)]
struct ShopUiEntity(Entity);

#[derive(Component)]
struct ShopEntities {
    variables: Entity,

    item_name_entity: Entity,
    item_description_entity: Entity,
    item_stats_list: Entity,

    amount_text_input: Entity,
    amount_slider: Entity,
    amount_max_text: Entity,

    current_money_text: Entity,
    delta_money_text: Entity,
    final_money_text: Entity,

    buy_sell_button: Entity,
}

struct SelectedItem {
    entry: ShopEntry,
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
    player_credits: Query<(Entity, &Credits), With<LocalPlayer>>,
) {
    let Ok((shop_ui, ui_ent)) = q_shop_ui.get_single() else {
        return;
    };

    let Ok((player_entity, credits)) = player_credits.get_single() else {
        error!("Missing credits on player?");
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

    let ui_variables_entity = commands
        .spawn((
            Name::new("UI variables"),
            NodeBundle {
                style: Style {
                    display: Display::None,
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            SelectedItemName::default(),
            SelectedItemDescription::default(),
            SelectedItemMaxQuantity::default(),
            NetCredits::default(),
            AmountSelected::default(),
            PricePerUnit::default(),
            ShopMode::Buy,
        ))
        .set_parent(ui_ent)
        .id();

    let mut shop_entities = ShopEntities {
        variables: ui_variables_entity,
        amount_max_text: Entity::PLACEHOLDER,
        amount_slider: Entity::PLACEHOLDER,
        amount_text_input: Entity::PLACEHOLDER,
        current_money_text: Entity::PLACEHOLDER,
        delta_money_text: Entity::PLACEHOLDER,
        final_money_text: Entity::PLACEHOLDER,
        item_description_entity: Entity::PLACEHOLDER,
        item_name_entity: Entity::PLACEHOLDER,
        item_stats_list: Entity::PLACEHOLDER,
        buy_sell_button: Entity::PLACEHOLDER,
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
                        shop_entities.item_name_entity = p
                            .spawn((
                                Name::new("Item Name"),
                                BindValues::<SelectedItemName>::new(vec![BindValue::new(
                                    ui_variables_entity,
                                    ReactableFields::Text { section: 0 },
                                )]),
                                TextBundle {
                                    text: Text::from_section("Select an item...", text_style.clone()),
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
                            ))
                            .id();

                        shop_entities.item_description_entity = p
                            .spawn((
                                Name::new("Description"),
                                BindValues::<SelectedItemDescription>::new(vec![BindValue::new(
                                    ui_variables_entity,
                                    ReactableFields::Text { section: 0 },
                                )]),
                                TextBundle {
                                    text: Text::from_section("", text_style_small.clone()),
                                    style: Style {
                                        margin: UiRect {
                                            bottom: Val::Px(30.0),
                                            top: Val::Px(10.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            ))
                            .id();

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

                        shop_entities.item_stats_list = p
                            .spawn(TextBundle {
                                text: Text::from_section("", text_style_small.clone()),
                                style: Style {
                                    margin: UiRect {
                                        left: Val::Px(20.0),
                                        bottom: Val::Px(10.0),
                                        top: Val::Px(10.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .id();
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
                                    price_per: _,
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
                        shop_entities.current_money_text = p
                            .spawn((
                                Name::new("Credits amount"),
                                BindValues::<Credits>::new(vec![BindValue::new(player_entity, ReactableFields::Text { section: 1 })]),
                                TextBundle {
                                    text: Text::from_sections([
                                        TextSection::new("$", text_style.clone()),
                                        TextSection::new(format!("{}", credits.amount()), text_style.clone()),
                                    ]),
                                    ..Default::default()
                                },
                            ))
                            .id();
                    });

                    shop_entities.delta_money_text = p
                        .spawn((
                            BindValues::<PricePerUnit>::new(vec![BindValue::new(
                                ui_variables_entity,
                                ReactableFields::Text { section: 1 },
                            )]),
                            BindValues::<AmountSelected>::new(vec![BindValue::new(
                                ui_variables_entity,
                                ReactableFields::Text { section: 3 },
                            )]),
                            TextBundle {
                                text: Text::from_sections([
                                    TextSection::new("- ", text_style.clone()),
                                    TextSection::new("", text_style.clone()),
                                    TextSection::new(" x ", text_style.clone()),
                                    TextSection::new("", text_style.clone()),
                                ]),
                                style: Style {
                                    bottom: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .id();

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
                        shop_entities.final_money_text = p
                            .spawn((
                                BindValues::<NetCredits>::new(vec![BindValue::new(
                                    ui_variables_entity,
                                    ReactableFields::Text { section: 1 },
                                )]),
                                TextBundle {
                                    text: Text::from_sections([
                                        TextSection::new("$", text_style.clone()),
                                        TextSection::new("", text_style.clone()),
                                    ]),
                                    ..Default::default()
                                },
                            ))
                            .id();
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
                        shop_entities.amount_text_input = p
                            .spawn((
                                Name::new("Amount Input"),
                                BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                                BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Max)]),
                                TextInputBundle {
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
                                },
                            ))
                            .id();

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

                                shop_entities.amount_max_text = p
                                    .spawn((
                                        BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(
                                            ui_variables_entity,
                                            ReactableFields::Text { section: 0 },
                                        )]),
                                        TextBundle {
                                            text: Text::from_section("", text_style_small.clone()),
                                            ..Default::default()
                                        },
                                    ))
                                    .id();
                            });

                            shop_entities.amount_slider = p
                                .spawn((
                                    Name::new("Amount slider"),
                                    ShopUiEntity(ui_ent),
                                    BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                                    BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(
                                        ui_variables_entity,
                                        ReactableFields::Max,
                                    )]),
                                    SliderBundle {
                                        node_bundle: NodeBundle {
                                            style: Style { ..Default::default() },
                                            ..Default::default()
                                        },
                                        slider: Slider {
                                            min: 0,
                                            max: 1,
                                            background_color: Color::hex("999999").unwrap(),
                                            foreground_color: Color::AQUAMARINE,
                                            square_color: Color::hex("555555").unwrap(),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                ))
                                .id();
                        });
                    });

                    shop_entities.buy_sell_button = p
                        .spawn(ButtonBundle::<BuyBtnEvent> {
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
                        })
                        .id();
                });
            });
        });

    debug_assert!(shop_entities.amount_max_text != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.amount_slider != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.amount_text_input != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.current_money_text != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.delta_money_text != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.final_money_text != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.item_description_entity != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.item_name_entity != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.item_stats_list != Entity::PLACEHOLDER);
    debug_assert!(shop_entities.buy_sell_button != Entity::PLACEHOLDER);

    commands.entity(ui_ent).insert(shop_entities);
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

fn click_item_event(
    mut ev_reader: EventReader<ClickItemEvent>,
    q_shop_entry: Query<(&ShopEntry, &ShopUiEntity)>,
    mut q_shop: Query<(&mut ShopUi, &ShopEntities)>,
    mut q_slider_value: Query<&mut SliderValue>,
) {
    for ev in ev_reader.read() {
        let Ok((entry, shop_ui_ent)) = q_shop_entry.get(ev.0) else {
            error!("Shop item button didn't have shop entry or shop ui entity?");
            return;
        };

        let Ok((mut shop_ui, shop_entities)) = q_shop.get_mut(shop_ui_ent.0) else {
            error!("Shop item button had invalid shop ui entity?");
            return;
        };

        q_slider_value.get_mut(shop_entities.amount_slider).unwrap().set_value(0);

        if shop_ui.selected_item.as_ref().map(|x| x.entry != *entry).unwrap_or(true) {
            shop_ui.selected_item = Some(SelectedItem { entry: *entry });
        }
    }
}

fn on_change_selected_item(
    items: Res<Registry<Item>>,
    langs: Res<Lang<Item>>,
    q_shop_changed: Query<(&ShopUi, &ShopEntities), Changed<ShopUi>>,
    player_credits: Query<&Credits, With<LocalPlayer>>,
    mut vars: Query<(
        &mut AmountSelected,
        &mut SelectedItemName,
        &mut SelectedItemDescription,
        &mut SelectedItemMaxQuantity,
        &mut NetCredits,
        &mut PricePerUnit,
        &mut ShopMode,
    )>,
) {
    for (shop_ui, shop_entities) in &q_shop_changed {
        let Some(selected_item) = &shop_ui.selected_item else {
            continue;
        };

        let credits = player_credits.get_single().copied().unwrap_or_default();

        let Ok((
            mut amount_selected,
            mut selected_item_name,
            mut selected_item_description,
            mut selected_item_max_quantity,
            mut net_credits,
            mut shop_price_per,
            mut shop_mode,
        )) = vars.get_mut(shop_entities.variables)
        else {
            continue;
        };

        amount_selected.0 = 0;
        net_credits.0 = credits.amount() as i64;

        let item_id = match selected_item.entry {
            ShopEntry::Buying {
                item_id,
                max_quantity_buying,
                price_per,
            } => {
                selected_item_max_quantity.0 = max_quantity_buying.unwrap_or(10000);
                shop_price_per.0 = price_per;
                *shop_mode = ShopMode::Buy;

                item_id
            }
            ShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } => {
                selected_item_max_quantity.0 = max_quantity_selling;
                shop_price_per.0 = price_per;
                *shop_mode = ShopMode::Sell;

                item_id
            }
        };

        let item = items.from_numeric_id(item_id);
        let item_name = langs.get_name(item).unwrap_or(item.unlocalized_name());

        selected_item_name.0 = item_name.to_owned();
        selected_item_description.0 = format!("Description for {item_name}");
    }
}

fn update_total(
    q_credits: Query<&Credits, With<LocalPlayer>>,
    mut q_changed_amount_selected: Query<(&AmountSelected, &PricePerUnit, &ShopMode, &mut NetCredits), Changed<AmountSelected>>,
) {
    for (amount_selected, price_per_unit, shop_mode, mut net_credits) in q_changed_amount_selected.iter_mut() {
        let Ok(credits) = q_credits.get_single() else {
            continue;
        };

        match *shop_mode {
            ShopMode::Buy => {
                net_credits.0 = credits.amount() as i64 - (price_per_unit.0 as u64 * amount_selected.0) as i64;
            }
            ShopMode::Sell => {
                net_credits.0 = credits.amount() as i64 + (price_per_unit.0 as u64 * amount_selected.0) as i64;
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<AmountSelected>(app);
    add_reactable_type::<AmountSelected>(app);
    add_reactable_type::<SelectedItemName>(app);
    add_reactable_type::<SelectedItemDescription>(app);
    add_reactable_type::<SelectedItemMaxQuantity>(app);
    add_reactable_type::<NetCredits>(app);
    add_reactable_type::<PricePerUnit>(app);
    add_reactable_type::<ShopMode>(app);

    register_button::<ClickSellTabEvent>(app);
    register_button::<ClickBuyTabEvent>(app);
    register_button::<BuyBtnEvent>(app);
    register_button::<ClickItemEvent>(app);

    app.add_mut_event::<OpenShopUiEvent>()
        .add_systems(
            Update,
            (
                open_shop_ui,
                click_item_event,
                on_change_selected_item,
                update_total,
                render_shop_ui,
            )
                .chain()
                .after(NetworkingSystemsSet::FlushReceiveMessages)
                .before(UiSystemSet::ApplyDeferredA),
        )
        .register_type::<AmountSelected>()
        .register_type::<SelectedItemName>()
        .register_type::<SelectedItemDescription>()
        .register_type::<SelectedItemMaxQuantity>()
        .register_type::<NetCredits>()
        .register_type::<PricePerUnit>()
        .register_type::<ShopMode>();
}
