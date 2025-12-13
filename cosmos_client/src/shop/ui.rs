use bevy::{color::palettes::css, prelude::*};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    economy::Credits,
    ecs::{
        NeedsDespawned,
        mut_events::{MutMessage, MutMessagesCommand},
    },
    inventory::Inventory,
    item::Item,
    netty::{NettyChannelClient, client::LocalPlayer, cosmos_encoder},
    registry::{Registry, identifiable::Identifiable},
    shop::{Shop, ShopEntry, netty::ClientShopMessages},
    state::GameState,
    structure::structure_block::StructureBlock,
};

use crate::{
    lang::Lang,
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            Disabled,
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            focus::OnSpawnFocus,
            scollable_container::ScrollBox,
            slider::Slider,
            text_input::{InputType, TextInput},
            window::GuiWindow,
        },
        font::DefaultFont,
        item_renderer::RenderItem,
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

use super::{PurchasedMessage, SoldMessage};

#[derive(Message)]
pub(super) struct OpenShopUiMessage {
    pub shop: Shop,
    pub structure_block: StructureBlock,
}

#[derive(Component, Debug)]
struct ShopUi {
    shop: Shop,
    /// # ⚠️ WARNING ⚠️
    ///
    /// This refers to the server's entity NOT the client's
    structure_block: StructureBlock,
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

fn open_shop_ui(
    mut commands: Commands,
    mut ev_reader: MessageReader<MutMessage<OpenShopUiMessage>>,
    q_open_shops: Query<Entity, With<ShopUi>>,
) {
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
            },
        ));
    }
}

fn render_shop_ui(
    mut commands: Commands,
    q_shop_ui: Query<(&ShopUi, Entity), Added<ShopUi>>,
    player_credits: Query<(Entity, &Credits), With<LocalPlayer>>,
    default_font: Res<DefaultFont>,
) {
    let Ok((shop_ui, ui_ent)) = q_shop_ui.single() else {
        return;
    };

    let Ok((player_entity, credits)) = player_credits.single() else {
        error!("Missing credits on player?");
        return;
    };

    let name = &shop_ui.shop.name;

    let text_style = TextFont {
        font_size: 32.0,
        font: default_font.clone(),
        ..Default::default()
    };

    let text_style_small = TextFont {
        font_size: 24.0,
        font: default_font.clone(),
        ..Default::default()
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
            BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
            Node {
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
            GuiWindow {
                title: name.into(),
                body_styles: Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn(Node {
                height: Val::Px(50.0),
                ..Default::default()
            })
            .with_children(|p| {
                p.spawn((
                    ShopUiEntity(ui_ent),
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    CosmosButton {
                        button_styles: Some(ButtonStyles {
                            background_color: Srgba::hex("880000").unwrap().into(),
                            hover_background_color: Srgba::hex("880000").unwrap().into(),
                            press_background_color: Srgba::hex("880000").unwrap().into(),
                            ..Default::default()
                        }),
                        text: Some(("Sell".into(), text_style.clone(), Default::default())),
                        ..Default::default()
                    },
                ))
                .observe(click_sell_tab);

                p.spawn((
                    ShopUiEntity(ui_ent),
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    CosmosButton {
                        button_styles: Some(ButtonStyles {
                            background_color: css::DARK_GREEN.into(),
                            hover_background_color: css::DARK_GREEN.into(),
                            press_background_color: css::DARK_GREEN.into(),
                            ..Default::default()
                        }),
                        text: Some(("Buy".into(), text_style.clone(), Default::default())),
                        ..Default::default()
                    },
                ))
                .observe(click_buy_tab);
            });

            p.spawn((
                Name::new("Body"),
                BorderColor::all(Srgba::hex("1C1C1C").unwrap()),
                Node {
                    border: UiRect {
                        bottom: Val::Px(4.0),
                        top: Val::Px(4.0),
                        ..Default::default()
                    },
                    flex_grow: 1.0,
                    ..Default::default()
                },
            ))
            .with_children(|body| {
                body.spawn((
                    Name::new("Main Stuff"),
                    Node {
                        flex_grow: 1.0,
                        padding: UiRect {
                            left: Val::Px(40.0),
                            right: Val::Px(40.0),
                            top: Val::Px(20.0),
                            bottom: Val::Px(20.0),
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|body| {
                    body.spawn((
                        Name::new("Description section"),
                        Node {
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
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
                            Text::new("Select an item..."),
                            text_style.clone(),
                            Node {
                                margin: UiRect {
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Name::new("Item picture"),
                            ShopRenderedItem,
                            Node {
                                width: Val::Px(128.0),
                                height: Val::Px(128.0),
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Name::new("Description"),
                            BindValues::<SelectedItemDescription>::new(vec![BindValue::new(
                                ui_variables_entity,
                                ReactableFields::Text { section: 0 },
                            )]),
                            Text::new(""),
                            text_style_small.clone(),
                            Node {
                                margin: UiRect {
                                    bottom: Val::Px(30.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Text::new("Stats"),
                            text_style.clone(),
                            Node {
                                margin: UiRect {
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));

                        p.spawn((
                            Text::new(""),
                            text_style_small.clone(),
                            Node {
                                margin: UiRect {
                                    left: Val::Px(20.0),
                                    bottom: Val::Px(10.0),
                                    top: Val::Px(10.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ));
                    });
                });

                body.spawn((
                    Name::new("Shop Categories"),
                    BackgroundColor(Srgba::hex("1C1C1C").unwrap().into()),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Px(500.0),
                        border: UiRect {
                            left: Val::Px(4.0),
                            ..Default::default()
                        },
                        padding: UiRect::new(Val::Px(10.0), Val::Px(0.0), Val::Px(10.0), Val::Px(10.0)),
                        ..Default::default()
                    },
                ))
                .with_children(|body| {
                    body.spawn((
                        Name::new("Stock Header Text"),
                        Label,
                        Text::new("Stock"),
                        text_style.clone(),
                        Node {
                            margin: UiRect {
                                bottom: Val::Px(10.0),
                                top: Val::Px(10.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ));

                    body.spawn((
                        Name::new("Search Text Box"),
                        OnSpawnFocus,
                        BindValues::<SearchItemQuery>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                        BorderColor::all(Srgba::hex("111111").unwrap()),
                        BackgroundColor(Srgba::hex("555555").unwrap().into()),
                        TextInput {
                            input_type: InputType::Text { max_length: Some(20) },
                            ..Default::default()
                        },
                        TextFont {
                            font_size: 24.0,
                            ..text_style.clone()
                        },
                        Node {
                            border: UiRect::all(Val::Px(2.0)),
                            padding: UiRect {
                                top: Val::Px(4.0),
                                bottom: Val::Px(4.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    ));

                    body.spawn((
                        Name::new("Items List"),
                        Node {
                            flex_grow: 1.0,
                            margin: UiRect::top(Val::Px(10.0)),
                            ..Default::default()
                        },
                        ScrollBox { ..Default::default() },
                    ))
                    .with_children(|p| {
                        shop_entities.contents_entity = p
                            .spawn((
                                Name::new("Contents"),
                                Node {
                                    padding: UiRect::all(Val::Px(10.0)),
                                    flex_direction: FlexDirection::Column,
                                    ..Default::default()
                                },
                            ))
                            .id();
                    });
                });
            });

            p.spawn((
                Name::new("Footer"),
                Node {
                    padding: UiRect::top(Val::Px(10.0)),
                    // height: Val::Px(170.0),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn(Node {
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
                })
                .with_children(|p| {
                    p.spawn(Node {
                        padding: UiRect {
                            left: Val::Px(20.0),
                            bottom: Val::Px(10.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|p| {
                        p.spawn((
                            Name::new("Credits amount"),
                            BindValues::<Credits>::new(vec![BindValue::new(player_entity, ReactableFields::Text { section: 1 })]),
                            Text::new("$"),
                            text_style.clone(),
                        ))
                        .with_children(|p| {
                            p.spawn((TextSpan::new(format!("{}", credits.amount())), text_style.clone()));
                        });
                    });

                    p.spawn((
                        BindValues::<ShopModeSign>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 0 })]),
                        BindValues::<PricePerUnit>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 1 })]),
                        BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 3 })]),
                        Text::new(""),
                        Node {
                            bottom: Val::Px(10.0),
                            ..Default::default()
                        },
                        text_style.clone(),
                    ))
                    .with_children(|p| {
                        p.spawn((TextSpan::new(""), text_style.clone()));
                        p.spawn((TextSpan::new(" x "), text_style.clone()));
                        p.spawn((TextSpan::new(""), text_style.clone()));
                    });

                    p.spawn((
                        BorderColor::all(Srgba::hex("555555").unwrap()),
                        Node {
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
                    ))
                    .with_children(|p| {
                        p.spawn((
                            BindValues::<NetCredits>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Text { section: 1 })]),
                            Text::new("$"),
                            text_style.clone(),
                        ))
                        .with_children(|p| {
                            p.spawn((TextSpan::new(""), text_style.clone()));
                        });
                    });
                });

                p.spawn(Node {
                    flex_grow: 3.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                })
                .with_children(|p| {
                    p.spawn(Node { ..Default::default() }).with_children(|p| {
                        p.spawn((
                            Name::new("Amount Input"),
                            BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                            BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Max)]),
                            BorderColor::all(Srgba::hex("111111").unwrap()),
                            BackgroundColor(Srgba::hex("555555").unwrap().into()),
                            Node {
                                width: Val::Px(250.0),
                                padding: UiRect::all(Val::Px(10.0)),
                                ..Default::default()
                            },
                            TextInput {
                                input_type: InputType::Integer { min: 0, max: 1000 },
                                ..Default::default()
                            },
                            TextFont {
                                font_size: 24.0,
                                ..text_style.clone()
                            },
                        ));

                        p.spawn(Node {
                            flex_grow: 1.0,
                            margin: UiRect {
                                right: Val::Px(10.0),
                                left: Val::Px(20.0),
                                ..Default::default()
                            },
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::SpaceBetween,
                            ..Default::default()
                        })
                        .with_children(|p| {
                            p.spawn(Node {
                                flex_grow: 1.0,
                                justify_content: JustifyContent::SpaceBetween,
                                ..Default::default()
                            })
                            .with_children(|p| {
                                p.spawn((Text::new("0"), text_style_small.clone()));

                                p.spawn((
                                    BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(
                                        ui_variables_entity,
                                        ReactableFields::Text { section: 0 },
                                    )]),
                                    Text::new(""),
                                    text_style_small.clone(),
                                ));
                            });

                            p.spawn((
                                Name::new("Amount slider"),
                                ShopUiEntity(ui_ent),
                                BindValues::<AmountSelected>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Value)]),
                                BindValues::<SelectedItemMaxQuantity>::new(vec![BindValue::new(ui_variables_entity, ReactableFields::Max)]),
                                Slider {
                                    min: 0,
                                    max: 1,
                                    background_color: Srgba::hex("999999").unwrap().into(),
                                    foreground_color: css::AQUAMARINE.into(),
                                    square_color: Srgba::hex("555555").unwrap().into(),
                                    ..Default::default()
                                },
                            ));
                        });
                    });

                    shop_entities.buy_sell_button = p
                        .spawn((
                            BuyOrSellButton { shop_entity: ui_ent },
                            Node {
                                margin: UiRect::top(Val::Px(10.0)),
                                height: Val::Px(80.0),
                                ..Default::default()
                            },
                            CosmosButton {
                                text: Some(("BUY".into(), text_style.clone(), Default::default())),
                                button_styles: Some(ButtonStyles {
                                    background_color: Srgba::hex("008000").unwrap().into(),
                                    hover_background_color: css::DARK_GREEN.into(),
                                    press_background_color: css::DARK_GREEN.into(),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                        ))
                        .observe(on_buy)
                        .id();
                });
            });
        });

    debug_assert!(shop_entities.buy_sell_button != Entity::PLACEHOLDER);

    commands.entity(ui_ent).insert(shop_entities);
}

#[derive(Component)]
struct BuyOrSellButton {
    shop_entity: Entity,
}

#[derive(Component)]
struct PrevClickedEntity(Entity);

#[derive(Component)]
struct ShopRenderedItem;

fn click_item_event(
    ev: On<ButtonEvent>,
    q_shop_entry: Query<(&ShopEntry, &ShopUiEntity)>,
    mut q_shop: Query<(&mut ShopUi, Option<&PrevClickedEntity>)>,
    mut q_background_color: Query<&mut BackgroundColor>,
    q_rendered_item: Query<Entity, With<ShopRenderedItem>>,
    mut commands: Commands,
) {
    let Ok((entry, shop_ui_ent)) = q_shop_entry.get(ev.0) else {
        error!("Shop item button didn't have shop entry or shop ui entity?");
        return;
    };

    let Ok((mut shop_ui, prev_clicked)) = q_shop.get_mut(shop_ui_ent.0) else {
        error!("Shop item button had invalid shop ui entity?");
        return;
    };

    if let Some(prev_clicked) = &prev_clicked
        && let Ok(mut background_color) = q_background_color.get_mut(prev_clicked.0)
    {
        *background_color = Color::NONE.into();
    }

    commands.entity(shop_ui_ent.0).insert(PrevClickedEntity(ev.0));
    if let Ok(mut background_color) = q_background_color.get_mut(ev.0) {
        *background_color = css::AQUAMARINE.into();
    }

    if shop_ui.selected_item.as_ref().map(|x| x.entry != *entry).unwrap_or(true) {
        shop_ui.selected_item = Some(SelectedItem { entry: *entry });
    }

    if let Ok(rendered_item) = q_rendered_item.single() {
        let item_id = match entry {
            ShopEntry::Buying {
                item_id,
                max_quantity_buying: _,
                price_per: _,
            } => *item_id,
            ShopEntry::Selling {
                item_id,
                max_quantity_selling: _,
                price_per: _,
            } => *item_id,
        };
        commands.entity(rendered_item).insert(RenderItem { item_id });
    }
}

fn on_change_selected_item(
    items: Res<Registry<Item>>,
    langs: Res<Lang<Item>>,
    q_changed_credits: Query<(), (With<LocalPlayer>, Or<(Changed<Credits>, Changed<Inventory>)>)>,
    q_changed_shop_ui: Query<(), Changed<ShopUi>>,
    q_shop: Query<(&ShopUi, &ShopEntities)>,
    q_player: Query<(&Credits, &Inventory), With<LocalPlayer>>,
    mut vars: Query<(
        &mut AmountSelected,
        &mut SelectedItemName,
        &mut SelectedItemDescription,
        &mut SelectedItemMaxQuantity,
        &mut PricePerUnit,
    )>,
) {
    if q_changed_credits.is_empty() && q_changed_shop_ui.is_empty() {
        return;
    }

    for (shop_ui, shop_entities) in &q_shop {
        let Some(selected_item) = &shop_ui.selected_item else {
            continue;
        };

        let Ok((credits, inventory)) = q_player.single() else {
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
                let items_of_this_type = inventory
                    .iter()
                    .flatten()
                    .filter(|x| x.item_id() == item_id)
                    .map(|x| x.quantity() as u32)
                    .sum::<u32>();

                selected_item_max_quantity.0 = max_quantity_buying.unwrap_or(10000).min(items_of_this_type);
                shop_price_per.0 = price_per;

                item_id
            }
            ShopEntry::Selling {
                item_id,
                max_quantity_selling,
                price_per,
            } => {
                selected_item_max_quantity.0 = max_quantity_selling.min(if price_per != 0 {
                    credits.amount() as u32 / price_per
                } else {
                    10000
                });
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
        let Ok(credits) = q_credits.single() else {
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
    default_font: Res<DefaultFont>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
) {
    for (ui_ent, shop_ents, shop_ui, shop_mode, search_item_query) in &q_search {
        let text_style_small = TextFont {
            font_size: 24.0,
            font: default_font.0.clone(),
            ..Default::default()
        };

        commands
            .entity(shop_ents.contents_entity)
            .despawn_related::<Children>()
            .with_children(|p| {
                let search = search_item_query.0.to_lowercase();

                for shop_entry in shop_ui.shop.contents.iter() {
                    let (item_id, price_per) = match *shop_mode {
                        ShopMode::Buy => {
                            let ShopEntry::Selling {
                                item_id,
                                max_quantity_selling: _,
                                price_per,
                            } = shop_entry
                            else {
                                continue;
                            };

                            (*item_id, *price_per)
                        }
                        ShopMode::Sell => {
                            let ShopEntry::Buying {
                                item_id,
                                max_quantity_buying: _,
                                price_per,
                            } = shop_entry
                            else {
                                continue;
                            };

                            (*item_id, *price_per)
                        }
                    };

                    let item = items.from_numeric_id(item_id);
                    let display_name = lang.get_name(item).unwrap_or(item.unlocalized_name());

                    if !display_name.to_lowercase().contains(&search) {
                        continue;
                    }

                    let amount_display = format!("${price_per}");

                    p.spawn((
                        Name::new(display_name.to_owned()),
                        *shop_entry,
                        ShopUiEntity(ui_ent),
                        CosmosButton::default(),
                        Node {
                            flex_direction: FlexDirection::Row,
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..Default::default()
                        },
                    ))
                    .observe(click_item_event)
                    .with_children(|p| {
                        p.spawn((
                            Name::new("Item Name"),
                            Text::new(display_name),
                            text_style_small.clone(),
                            Node {
                                flex_grow: 1.0,
                                ..Default::default()
                            },
                        ));
                        p.spawn((
                            Name::new("Price"),
                            Text::new(format!("({amount_display})")),
                            text_style_small.clone(),
                        ));
                    });
                }
            });
    }
}

fn enable_buy_button(
    mut commands: Commands,
    mut q_shop_ui: Query<&mut ShopUi>,
    q_buy_button: Query<(Entity, &BuyOrSellButton), With<CosmosButton>>,
    mut ev_reader: MessageReader<PurchasedMessage>,
) {
    for ev in ev_reader.read() {
        for (entity, buy_button) in q_buy_button.iter() {
            let Ok(mut shop_ui) = q_shop_ui.get_mut(buy_button.shop_entity) else {
                continue;
            };

            if shop_ui.structure_block.structure() == ev.structure_entity && shop_ui.structure_block.coords() == ev.shop_block {
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
    q_buy_button: Query<(Entity, &BuyOrSellButton), With<CosmosButton>>,
    mut ev_reader: MessageReader<SoldMessage>,
) {
    for ev in ev_reader.read() {
        for (entity, buy_button) in q_buy_button.iter() {
            let Ok(mut shop_ui) = q_shop_ui.get_mut(buy_button.shop_entity) else {
                continue;
            };

            if shop_ui.structure_block.structure() == ev.structure_entity && shop_ui.structure_block.coords() == ev.shop_block {
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
    ev: On<ButtonEvent>,
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    q_shop_ui: Query<(&ShopUi, &AmountSelected)>,
    q_buy_button: Query<&BuyOrSellButton>,
) {
    let Ok(buy_button) = q_buy_button.get(ev.0) else {
        error!("Buy button event missing buy button entity");
        return;
    };

    let Ok((shop_ui, amount_selected)) = q_shop_ui.get(buy_button.shop_entity) else {
        return;
    };

    let Some(selected_item) = &shop_ui.selected_item else {
        return;
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
                    structure_entity: shop_ui.structure_block.structure(),
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
                    structure_entity: shop_ui.structure_block.structure(),
                    item_id,
                    quantity: amount_selected.0 as u32,
                }),
            );
        }
    }
}

fn click_buy_tab(ev: On<ButtonEvent>, mut q_shop_mode: Query<&mut ShopMode>, q_shop_ui_entity: Query<&ShopUiEntity>) {
    let Ok(shop_ui_ent) = q_shop_ui_entity.get(ev.0) else {
        return;
    };

    let Ok(mut shop_mode) = q_shop_mode.get_mut(shop_ui_ent.0) else {
        return;
    };

    if *shop_mode != ShopMode::Buy {
        *shop_mode = ShopMode::Buy;
    }
}

fn click_sell_tab(ev: On<ButtonEvent>, mut q_shop_mode: Query<&mut ShopMode>, q_shop_ui_entity: Query<&ShopUiEntity>) {
    let Ok(shop_ui_ent) = q_shop_ui_entity.get(ev.0) else {
        return;
    };

    let Ok(mut shop_mode) = q_shop_mode.get_mut(shop_ui_ent.0) else {
        return;
    };

    if *shop_mode != ShopMode::Sell {
        *shop_mode = ShopMode::Sell;
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
    mut q_button: Query<&mut CosmosButton>,
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

    app.configure_sets(
        Update,
        ShopLogicSet::ShopLogic
            .before(UiSystemSet::PreDoUi)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_mut_event::<OpenShopUiMessage>()
        .add_systems(
            Update,
            (
                open_shop_ui,
                on_change_shop_mode,
                on_change_selected_item,
                update_total,
                update_search,
                render_shop_ui,
                enable_buy_button,
                enable_sell_button,
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
