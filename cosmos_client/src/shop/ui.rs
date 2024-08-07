use bevy::{
    app::{App, Update},
    asset::AssetServer,
    color::{palettes::css, Color, Srgba},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        query::{Added, Changed, Or, With},
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::{BuildChildren, DespawnRecursiveExt},
    log::{error, info},
    reflect::Reflect,
    state::condition::in_state,
    text::{Text, TextSection, TextStyle},
    ui::{
        node_bundles::{NodeBundle, TextBundle},
        widget::Label,
        BackgroundColor, FlexDirection, JustifyContent, Style, UiRect, Val,
    },
};
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    economy::Credits,
    ecs::{
        mut_events::{MutEvent, MutEventsCommand},
        NeedsDespawned,
    },
    item::Item,
    netty::{client::LocalPlayer, cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    registry::{identifiable::Identifiable, Registry},
    shop::{netty::ClientShopMessages, Shop, ShopEntry},
    structure::structure_block::StructureBlock,
};

use crate::{
    lang::Lang,
    state::game_state::GameState,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::{ScrollBox, ScrollBundle},
            slider::{Slider, SliderBundle},
            text_input::{InputType, TextInput, TextInputBundle},
            window::{GuiWindow, WindowBundle},
            Disabled,
        },
        reactivity::{add_reactable_type, BindValue, BindValues, ReactableFields, ReactableValue},
        OpenMenu, UiSystemSet,
    },
};

use super::{PurchasedEvent, SoldEvent};

#[derive(Event)]
pub(super) struct OpenShopUiEvent {
    pub shop: Shop,
    pub structure_entity: Entity,
    pub structure_block: StructureBlock,
}

#[derive(Component, Debug)]
struct ShopUi {
    shop: Shop,
    structure_block: StructureBlock,
    /// # ⚠️ WARNING ⚠️
    ///
    /// This refers to the server's entity NOT the client's
    structure_entity: Entity,
    selected_item: Option<SelectedItem>,
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SelectedItemName(String);

impl ReactableValue for SelectedItemName {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        new_value.clone_into(&mut self.0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SearchItemQuery(String);

impl ReactableValue for SearchItemQuery {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        new_value.clone_into(&mut self.0);
    }
}

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct SelectedItemDescription(String);

impl ReactableValue for SelectedItemDescription {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        new_value.clone_into(&mut self.0);
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

#[derive(Reflect, Component, PartialEq, Eq, Default)]
struct ShopModeSign(String);

impl ReactableValue for ShopModeSign {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        new_value.clone_into(&mut self.0);
    }
}

#[derive(Component)]
struct ShopUiEntity(Entity);

#[derive(Component)]
struct ShopEntities {
    variables: Entity,
    contents_entity: Entity,
    buy_sell_button: Entity,
}

#[derive(Debug)]
struct SelectedItem {
    entry: ShopEntry,
}

fn open_shop_ui(mut commands: Commands, mut ev_reader: EventReader<MutEvent<OpenShopUiEvent>>, q_open_shops: Query<Entity, With<ShopUi>>) {
    for ev in ev_reader.read() {
        let mut ev = ev.write();
        let shop = std::mem::take(&mut ev.shop);

        for ent in q_open_shops.iter() {
            commands.entity(ent).insert(NeedsDespawned);
        }

        commands.spawn((
            OpenMenu::new(0),
            ShopUi {
                shop,
                selected_item: None,
                structure_block: ev.structure_block,
                structure_entity: ev.structure_entity,
            },
        ));
    }
}

fn render_shop_ui(
    mut commands: Commands,
    q_shop_ui: Query<(&ShopUi, Entity), Added<ShopUi>>,
    asset_server: Res<AssetServer>,
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
        color: css::WHITE.into(),
        font_size: 32.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let text_style_small = TextStyle {
        color: css::WHITE.into(),
        font_size: 24.0,
        font: asset_server.load("fonts/PixeloidSans.ttf"),
    };

    let ui_variables_entity = ui_ent;

    let mut shop_entities = ShopEntities {
        variables: ui_variables_entity,
        contents_entity: Entity::PLACEHOLDER,
        buy_sell_button: Entity::PLACEHOLDER,
    };

    commands
        .entity(ui_ent)
        .insert((
            SelectedItemName::default(),
            SelectedItemDescription::default(),
            SelectedItemMaxQuantity::default(),
            NetCredits::default(),
            AmountSelected::default(),
            PricePerUnit::default(),
            SearchItemQuery::default(),
            ShopModeSign("- $".into()),
            ShopMode::Buy,
        ))
        .insert((
            Name::new("Shop UI"),
            WindowBundle {
                node_bundle: NodeBundle {
                    background_color: Srgba::hex("2D2D2D").unwrap().into(),
                    style: Style {
                        width: Val::Px(1000.0),
                        height: Val::Px(800.0),
                        margin: UiRect {
                            // Centers it vertically
                            top: Val::Auto,
                            bottom: Val::Auto,
                            left: Val::Auto,
                            right: Val::Auto,
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
                },
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn(NodeBundle {
                style: Style {
                    height: Val::Px(50.0),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|p| {
                p.spawn((
                    ShopUiEntity(ui_ent),
                    ButtonBundle::<ClickSellTabEvent> {
                        node_bundle: NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        button: Button {
                            button_styles: Some(ButtonStyles {
                                background_color: Srgba::hex("880000").unwrap().into(),
                                hover_background_color: Srgba::hex("880000").unwrap().into(),
                                press_background_color: Srgba::hex("880000").unwrap().into(),
                                ..Default::default()
                            }),
                            text: Some(("Sell".into(), text_style.clone())),
                            ..Default::default()
                        },
                    },
                ));

                p.spawn((
                    ShopUiEntity(ui_ent),
                    ButtonBundle::<ClickBuyTabEvent> {
                        node_bundle: NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        button: Button {
                            button_styles: Some(ButtonStyles {
                                background_color: css::DARK_GREEN.into(),
                                hover_background_color: css::DARK_GREEN.into(),
                                press_background_color: css::DARK_GREEN.into(),
                                ..Default::default()
                            }),
                            text: Some(("Buy".into(), text_style.clone())),
                            ..Default::default()
                        },
                    },
                ));
            });

            p.spawn((
                Name::new("Body"),
                NodeBundle {
                    border_color: Srgba::hex("1C1C1C").unwrap().into(),
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
                        p.spawn((
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
                        ));

                        p.spawn((
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
                        ));

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

                        p.spawn(TextBundle {
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
                        });
                    });

                    body.spawn((Name::new("Item picture"), NodeBundle { ..Default::default() }));
                });

                body.spawn((
                    Name::new("Shop Categories"),
                    NodeBundle {
                        border_color: Srgba::hex("1C1C1C").unwrap().into(),
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
                        BindValues::<SearchItemQuery>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                        TextInputBundle {
                            text_input: TextInput {
                                style: text_style.clone(),
                                input_type: InputType::Text { max_length: Some(20) },
                                ..Default::default()
                            },
                            node_bundle: NodeBundle {
                                border_color: Srgba::hex("111111").unwrap().into(),
                                background_color: Srgba::hex("555555").unwrap().into(),
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
                        },
                    ))
                    .with_children(|p| {
                        shop_entities.contents_entity = p
                            .spawn((
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
                            .id();
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
                        // WIERDNESS:
                        width: Val::Percent(0.0),
                        flex_grow: 4.0,
                        /*
                           Explanation:

                           Idk why it works like this, but if I make width auto and flex_grow: 2.0 (which are what they are supposed to be),
                           when the text changes its length the container width changes, which it shouldnt.

                           However, by randomly guessing i found that a width of 0 and flex_grow of 4.0 (???) makes it look like
                           flex_grow 2.0 and its width isn't effected by the size of the text. Idk why.
                        */
                        // END WEIRDNESS
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
                        p.spawn((
                            Name::new("Credits amount"),
                            BindValues::<Credits>::new(vec![BindValue::new(player_entity, ReactableFields::Text { section: 1 })]),
                            TextBundle {
                                text: Text::from_sections([
                                    TextSection::new("$", text_style.clone()),
                                    TextSection::new(format!("{}", credits.amount()), text_style.clone()),
                                ]),
                                ..Default::default()
                            },
                        ));
                    });

                    p.spawn((
                        BindValues::<ShopModeSign>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 0 })]),
                        BindValues::<PricePerUnit>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 1 })]),
                        BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 3 })]),
                        TextBundle {
                            text: Text::from_sections([
                                TextSection::new("", text_style.clone()),
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
                    ));

                    p.spawn(NodeBundle {
                        border_color: Srgba::hex("555555").unwrap().into(),
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
                        p.spawn((
                            BindValues::<NetCredits>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 1 })]),
                            TextBundle {
                                text: Text::from_sections([
                                    TextSection::new("$", text_style.clone()),
                                    TextSection::new("", text_style.clone()),
                                ]),
                                ..Default::default()
                            },
                        ));
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
                        p.spawn((
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
                                    border_color: Srgba::hex("111111").unwrap().into(),
                                    background_color: Srgba::hex("555555").unwrap().into(),
                                    ..Default::default()
                                },
                                text_input: TextInput {
                                    input_type: InputType::Integer { min: 0, max: 1000 },
                                    style: text_style.clone(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));

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

                                p.spawn((
                                    BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(
                                        ui_variables_entity,
                                        ReactableFields::Text { section: 0 },
                                    )]),
                                    TextBundle {
                                        text: Text::from_section("", text_style_small.clone()),
                                        ..Default::default()
                                    },
                                ));
                            });

                            p.spawn((
                                Name::new("Amount slider"),
                                ShopUiEntity(ui_ent),
                                BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                                BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Max)]),
                                SliderBundle {
                                    node_bundle: NodeBundle {
                                        style: Style { ..Default::default() },
                                        ..Default::default()
                                    },
                                    slider: Slider {
                                        min: 0,
                                        max: 1,
                                        background_color: Srgba::hex("999999").unwrap().into(),
                                        foreground_color: css::AQUAMARINE.into(),
                                        square_color: Srgba::hex("555555").unwrap().into(),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            ));
                        });
                    });

                    shop_entities.buy_sell_button = p
                        .spawn((
                            BuyOrSellButton { shop_entity: ui_ent },
                            ButtonBundle::<BuyOrSellBtnEvent> {
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
                                        background_color: Srgba::hex("008000").unwrap().into(),
                                        hover_background_color: css::DARK_GREEN.into(),
                                        press_background_color: css::DARK_GREEN.into(),
                                        ..Default::default()
                                    }),
                                    ..Default::default()
                                },
                            },
                        ))
                        .id();
                });
            });
        });

    debug_assert!(shop_entities.buy_sell_button != Entity::PLACEHOLDER);

    commands.entity(ui_ent).insert(shop_entities);
}

#[derive(Event, Debug)]
struct ClickSellTabEvent(Entity);

impl ButtonEvent for ClickSellTabEvent {
    fn create_event(e: Entity) -> Self {
        Self(e)
    }
}

#[derive(Event, Debug)]
struct ClickBuyTabEvent(Entity);

impl ButtonEvent for ClickBuyTabEvent {
    fn create_event(e: Entity) -> Self {
        Self(e)
    }
}

#[derive(Component)]
struct BuyOrSellButton {
    shop_entity: Entity,
}

#[derive(Event, Debug)]
struct BuyOrSellBtnEvent(Entity);

impl ButtonEvent for BuyOrSellBtnEvent {
    fn create_event(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Event, Debug)]
struct ClickItemEvent(Entity);

impl ButtonEvent for ClickItemEvent {
    fn create_event(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Component)]
struct PrevClickedEntity(Entity);

fn click_item_event(
    mut ev_reader: EventReader<ClickItemEvent>,
    q_shop_entry: Query<(&ShopEntry, &ShopUiEntity)>,
    mut q_shop: Query<(&mut ShopUi, Option<&PrevClickedEntity>)>,
    mut q_background_color: Query<&mut BackgroundColor>,
    mut commands: Commands,
) {
    for ev in ev_reader.read() {
        let Ok((entry, shop_ui_ent)) = q_shop_entry.get(ev.0) else {
            error!("Shop item button didn't have shop entry or shop ui entity?");
            return;
        };

        let Ok((mut shop_ui, prev_clicked)) = q_shop.get_mut(shop_ui_ent.0) else {
            error!("Shop item button had invalid shop ui entity?");
            return;
        };

        if let Some(prev_clicked) = &prev_clicked {
            if let Ok(mut background_color) = q_background_color.get_mut(prev_clicked.0) {
                *background_color = Color::NONE.into();
            }
        }

        commands.entity(shop_ui_ent.0).insert(PrevClickedEntity(ev.0));
        if let Ok(mut background_color) = q_background_color.get_mut(ev.0) {
            *background_color = css::AQUAMARINE.into();
        }

        if shop_ui.selected_item.as_ref().map(|x| x.entry != *entry).unwrap_or(true) {
            shop_ui.selected_item = Some(SelectedItem { entry: *entry });
        }
    }
}

fn on_change_selected_item(
    items: Res<Registry<Item>>,
    langs: Res<Lang<Item>>,
    q_shop_changed: Query<(&ShopUi, &ShopEntities), Changed<ShopUi>>,
    mut vars: Query<(
        &mut AmountSelected,
        &mut SelectedItemName,
        &mut SelectedItemDescription,
        &mut SelectedItemMaxQuantity,
        &mut PricePerUnit,
    )>,
) {
    for (shop_ui, shop_entities) in &q_shop_changed {
        let Some(selected_item) = &shop_ui.selected_item else {
            continue;
        };

        let Ok((
            mut amount_selected,
            mut selected_item_name,
            mut selected_item_description,
            mut selected_item_max_quantity,
            mut shop_price_per,
        )) = vars.get_mut(shop_entities.variables)
        else {
            continue;
        };

        let item_id = match selected_item.entry {
            ShopEntry::Buying {
                item_id,
                max_quantity_buying,
                price_per,
            } => {
                selected_item_max_quantity.0 = max_quantity_buying.unwrap_or(10000);
                shop_price_per.0 = price_per;

                item_id
            }
            ShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } => {
                selected_item_max_quantity.0 = max_quantity_selling;
                shop_price_per.0 = price_per;

                item_id
            }
        };

        amount_selected.0 = amount_selected.0.min(selected_item_max_quantity.0 as u64);

        let item = items.from_numeric_id(item_id);
        let item_name = langs.get_name(item).unwrap_or(item.unlocalized_name());

        item_name.clone_into(&mut selected_item_name.0);
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

fn update_search(
    q_search: Query<(Entity, &ShopEntities, &ShopUi, &ShopMode, &SearchItemQuery), Or<(Changed<SearchItemQuery>, Changed<ShopMode>)>>,
    mut commands: Commands,

    asset_server: Res<AssetServer>,

    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
) {
    for (ui_ent, shop_ents, shop_ui, shop_mode, search_item_query) in &q_search {
        let text_style_small = TextStyle {
            color: css::WHITE.into(),
            font_size: 24.0,
            font: asset_server.load("fonts/PixeloidSans.ttf"),
        };

        commands.entity(shop_ents.contents_entity).despawn_descendants().with_children(|p| {
            let search = search_item_query.0.to_lowercase();

            for shop_entry in shop_ui.shop.contents.iter() {
                let (item_id, max_quantity_selling) = match *shop_mode {
                    ShopMode::Buy => {
                        let ShopEntry::Selling {
                            item_id,
                            max_quantity_selling,
                            price_per: _,
                        } = shop_entry
                        else {
                            continue;
                        };

                        (*item_id, Some(*max_quantity_selling))
                    }
                    ShopMode::Sell => {
                        let ShopEntry::Buying {
                            item_id,
                            max_quantity_buying,
                            price_per: _,
                        } = shop_entry
                        else {
                            continue;
                        };

                        (*item_id, *max_quantity_buying)
                    }
                };

                let item = items.from_numeric_id(item_id);
                let display_name = lang.get_name(item).unwrap_or(item.unlocalized_name());

                if !display_name.to_lowercase().contains(&search) {
                    continue;
                }

                let amount_display = if let Some(max_quantity_selling) = max_quantity_selling {
                    format!("{max_quantity_selling}")
                } else {
                    "Unlimited".into()
                };

                p.spawn((
                    Name::new(display_name.to_owned()),
                    *shop_entry,
                    ShopUiEntity(ui_ent),
                    ButtonBundle::<ClickItemEvent> {
                        button: Button { ..Default::default() },
                        node_bundle: NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                margin: UiRect::vertical(Val::Px(2.0)),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
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
                            text: Text::from_section(format!("({amount_display})"), text_style_small.clone()),
                            ..Default::default()
                        },
                    ));
                });
            }
        });
    }
}

fn enable_buy_button(
    mut commands: Commands,
    mut q_shop_ui: Query<&mut ShopUi>,
    q_buy_button: Query<(Entity, &BuyOrSellButton), With<Button<BuyOrSellBtnEvent>>>,
    mut ev_reader: EventReader<PurchasedEvent>,
) {
    for ev in ev_reader.read() {
        for (entity, buy_button) in q_buy_button.iter() {
            let Ok(mut shop_ui) = q_shop_ui.get_mut(buy_button.shop_entity) else {
                continue;
            };

            if shop_ui.structure_entity == ev.structure_entity && shop_ui.structure_block.coords() == ev.shop_block {
                match &ev.details {
                    Ok(shop) => {
                        shop_ui.shop = shop.clone();
                        info!("Purchase successful!");
                    }
                    Err(err) => {
                        info!("{err:?}");
                    }
                };

                commands.entity(entity).remove::<Disabled>();
            }
        }
    }
}

fn enable_sell_button(
    mut commands: Commands,
    mut q_shop_ui: Query<&mut ShopUi>,
    q_buy_button: Query<(Entity, &BuyOrSellButton), With<Button<BuyOrSellBtnEvent>>>,
    mut ev_reader: EventReader<SoldEvent>,
) {
    for ev in ev_reader.read() {
        for (entity, buy_button) in q_buy_button.iter() {
            let Ok(mut shop_ui) = q_shop_ui.get_mut(buy_button.shop_entity) else {
                continue;
            };

            if shop_ui.structure_entity == ev.structure_entity && shop_ui.structure_block.coords() == ev.shop_block {
                match &ev.details {
                    Ok(shop) => {
                        shop_ui.shop = shop.clone();
                        info!("Sell successful!");
                    }
                    Err(err) => {
                        info!("{err:?}");
                    }
                };

                commands.entity(entity).remove::<Disabled>();
            }
        }
    }
}

fn on_buy(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    q_shop_ui: Query<(&ShopUi, &AmountSelected)>,
    q_buy_button: Query<&BuyOrSellButton>,
    mut ev_reader: EventReader<BuyOrSellBtnEvent>,
) {
    for ev in ev_reader.read() {
        let Ok(buy_button) = q_buy_button.get(ev.0) else {
            error!("Buy button event missing buy button entity");
            continue;
        };

        let Ok((shop_ui, amount_selected)) = q_shop_ui.get(buy_button.shop_entity) else {
            continue;
        };

        let Some(selected_item) = &shop_ui.selected_item else {
            continue;
        };

        // Prevent accidental duplicate purchases
        commands.entity(ev.0).insert(Disabled);

        match selected_item.entry {
            ShopEntry::Buying {
                item_id,
                max_quantity_buying: _,
                price_per: _,
            } => {
                client.send_message(
                    NettyChannelClient::Shop,
                    cosmos_encoder::serialize(&ClientShopMessages::Sell {
                        shop_block: shop_ui.structure_block.coords(),
                        structure_entity: shop_ui.structure_entity,
                        item_id,
                        quantity: amount_selected.0 as u32,
                    }),
                );
            }
            ShopEntry::Selling {
                item_id,
                max_quantity_selling: _,
                price_per: _,
            } => {
                client.send_message(
                    NettyChannelClient::Shop,
                    cosmos_encoder::serialize(&ClientShopMessages::Buy {
                        shop_block: shop_ui.structure_block.coords(),
                        structure_entity: shop_ui.structure_entity,
                        item_id,
                        quantity: amount_selected.0 as u32,
                    }),
                );
            }
        }
    }
}

fn click_buy_tab(
    mut q_shop_mode: Query<&mut ShopMode>,
    q_shop_ui_entity: Query<&ShopUiEntity>,
    mut ev_reader: EventReader<ClickBuyTabEvent>,
) {
    for ev in ev_reader.read() {
        let Ok(shop_ui_ent) = q_shop_ui_entity.get(ev.0) else {
            continue;
        };

        let Ok(mut shop_mode) = q_shop_mode.get_mut(shop_ui_ent.0) else {
            continue;
        };

        if *shop_mode != ShopMode::Buy {
            *shop_mode = ShopMode::Buy;
        }
    }
}

fn click_sell_tab(
    mut q_shop_mode: Query<&mut ShopMode>,
    q_shop_ui_entity: Query<&ShopUiEntity>,
    mut ev_reader: EventReader<ClickSellTabEvent>,
) {
    for ev in ev_reader.read() {
        let Ok(shop_ui_ent) = q_shop_ui_entity.get(ev.0) else {
            continue;
        };

        let Ok(mut shop_mode) = q_shop_mode.get_mut(shop_ui_ent.0) else {
            continue;
        };

        if *shop_mode != ShopMode::Sell {
            *shop_mode = ShopMode::Sell;
        }
    }
}

/*
SelectedItemName::default(),
SelectedItemDescription::default(),
SelectedItemMaxQuantity::default(),
NetCredits::default(),
AmountSelected::default(),
PricePerUnit::default(),
SearchItemQuery::default(),
ShopMode::Buy,
*/

fn on_change_shop_mode(
    mut q_shop: Query<
        (
            &ShopMode,
            &ShopEntities,
            &mut SelectedItemName,
            &mut SelectedItemDescription,
            &mut SelectedItemMaxQuantity,
            &mut PricePerUnit,
            &mut AmountSelected,
            &mut ShopModeSign,
            &mut ShopUi,
        ),
        Changed<ShopMode>,
    >,
    mut q_button: Query<&mut Button<BuyOrSellBtnEvent>>,
) {
    for (
        &shop_mode,
        shop_entities,
        mut selected_item_name,
        mut selected_item_desc,
        mut selected_item_max_qty,
        mut price_per_unit,
        mut amount_selected,
        mut shop_mode_sign,
        mut shop_ui,
    ) in q_shop.iter_mut()
    {
        shop_ui.selected_item = None;
        amount_selected.0 = 0;
        *selected_item_name = Default::default();
        *selected_item_desc = Default::default();
        *selected_item_max_qty = Default::default();
        *price_per_unit = Default::default();

        shop_mode_sign.0 = match shop_mode {
            ShopMode::Buy => "- $",
            ShopMode::Sell => "+ $",
        }
        .into();

        if let Ok(mut btn) = q_button.get_mut(shop_entities.buy_sell_button) {
            btn.text.as_mut().expect("Buy/sell has no text?").0 = match shop_mode {
                ShopMode::Buy => "BUY",
                ShopMode::Sell => "SELL",
            }
            .into();

            btn.button_styles = Some(match shop_mode {
                ShopMode::Buy => ButtonStyles {
                    background_color: css::DARK_GREEN.into(),
                    hover_background_color: css::DARK_GREEN.into(),
                    press_background_color: css::DARK_GREEN.into(),
                    ..Default::default()
                },
                ShopMode::Sell => ButtonStyles {
                    background_color: Srgba::hex("880000").unwrap().into(),
                    hover_background_color: Srgba::hex("880000").unwrap().into(),
                    press_background_color: Srgba::hex("880000").unwrap().into(),
                    ..Default::default()
                },
            });
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum ShopLogicSet {
    ShopLogic,
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<AmountSelected>(app);
    add_reactable_type::<SelectedItemName>(app);
    add_reactable_type::<SelectedItemDescription>(app);
    add_reactable_type::<SelectedItemMaxQuantity>(app);
    add_reactable_type::<NetCredits>(app);
    add_reactable_type::<PricePerUnit>(app);
    add_reactable_type::<ShopMode>(app);
    add_reactable_type::<SearchItemQuery>(app);
    add_reactable_type::<ShopModeSign>(app);

    register_button::<ClickSellTabEvent>(app);
    register_button::<ClickBuyTabEvent>(app);
    register_button::<BuyOrSellBtnEvent>(app);
    register_button::<ClickItemEvent>(app);

    app.configure_sets(
        Update,
        ShopLogicSet::ShopLogic
            .in_set(NetworkingSystemsSet::Between)
            .before(UiSystemSet::PreDoUi)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_mut_event::<OpenShopUiEvent>()
        .add_systems(
            Update,
            (
                open_shop_ui,
                click_buy_tab,
                click_sell_tab,
                on_change_shop_mode,
                click_item_event,
                on_change_selected_item,
                update_total,
                update_search,
                render_shop_ui,
                enable_buy_button,
                enable_sell_button,
                on_buy,
            )
                .in_set(ShopLogicSet::ShopLogic)
                .chain(),
        )
        .register_type::<AmountSelected>()
        .register_type::<SelectedItemName>()
        .register_type::<SelectedItemDescription>()
        .register_type::<SelectedItemMaxQuantity>()
        .register_type::<NetCredits>()
        .register_type::<PricePerUnit>()
        .register_type::<ShopMode>()
        .register_type::<SearchItemQuery>();
}
