use bevy::{
    color::palettes::css,
    picking::{hover::PickingInteraction, pointer::PointerPress},
    prelude::*,
};
use cosmos_core::{
    crafting::{
        blocks::basic_fabricator::CraftBasicFabricatorRecipeMessage,
        recipes::{
            RecipeItem,
            basic_fabricator::{BasicFabricatorCraftResultMessage, BasicFabricatorRecipe, BasicFabricatorRecipes, FabricatorItemInput},
        },
    },
    ecs::NeedsDespawned,
    inventory::Inventory,
    item::{Item, item_category::ItemCategory},
    netty::{client::LocalPlayer, sync::events::client_event::NettyMessageWriter},
    prelude::StructureBlock,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{
        OpenMenu,
        components::{
            button::{ButtonEvent, CosmosButton},
            show_cursor::ShowCursor,
            text_input::TextInput,
        },
        constants::CHECK,
        font::DefaultFont,
        item_renderer::{CustomHoverTooltip, NoHoverTooltip, RenderItem},
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

use super::{FabricatorMenuSet, OpenBasicFabricatorMenu};

#[derive(Component, Debug, Clone)]
struct Recipe(BasicFabricatorRecipe);

#[derive(Component, Debug)]
struct OpenBasicFabMenu(StructureBlock);

#[derive(Component, PartialEq, Eq)]
struct RecipeSearch(String);

impl ReactableValue for RecipeSearch {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.into();
    }
}

#[derive(Component)]
struct SwapToCategory(u16);

#[derive(Component)]
struct RecipesList(Option<u16>);

fn populate_menu(
    mut commands: Commands,
    q_added_menu: Query<(Entity, &OpenBasicFabricatorMenu), Added<OpenBasicFabricatorMenu>>,
    font: Res<DefaultFont>,
    crafting_recipes: Res<BasicFabricatorRecipes>,
    items: Res<Registry<Item>>,
    categories: Res<Registry<ItemCategory>>,
    category_lang: Res<Lang<ItemCategory>>,
) {
    for (ent, fab_menu) in q_added_menu.iter() {
        let mut ecmds = commands.entity(ent);

        let text_style = TextFont {
            font: font.0.clone(),
            font_size: 24.0,
            ..Default::default()
        };

        ecmds.insert((
            OpenMenu::new(0),
            BorderRadius::all(Val::Px(20.0)),
            // transparent aqua
            BackgroundColor(Srgba::hex("0099BB99").unwrap().into()),
            BorderColor::all(css::AQUA),
            OpenBasicFabMenu(fab_menu.0),
            Node {
                width: Val::Percent(80.0),
                height: Val::Percent(80.0),
                margin: UiRect::AUTO,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            ShowCursor,
            GlobalZIndex(1),
            RecipeSearch("".into()),
        ));

        let root_ent_id = ecmds.id();

        ecmds.with_children(|p| {
            // categories
            p.spawn((
                Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    border: UiRect::right(Val::Px(2.0)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    width: Val::Px(150.0),
                    ..Default::default()
                },
                BorderColor::all(css::DARK_GREY),
            ))
            .with_children(|p| {
                let mut categories = categories
                    .iter()
                    .filter(|c| {
                        crafting_recipes.iter().any(|r| {
                            items
                                .from_numeric_id(r.output.item)
                                .category()
                                .is_some_and(|n| n == c.unlocalized_name())
                        })
                    })
                    .collect::<Vec<_>>();
                categories.sort_by_key(|k| k.unlocalized_name());

                for category in categories {
                    p.spawn((
                        block_item_node(),
                        CustomHoverTooltip::new(category_lang.get_name_or_unlocalized(category)),
                        RenderItem {
                            item_id: items.from_id(category.item_icon_id()).map(|x| x.id()).unwrap_or_else(|| {
                                error!(
                                    "Invalid category item id {}! Rendering item at id 0 instead.",
                                    category.item_icon_id()
                                );
                                0
                            }),
                        },
                        SwapToCategory(category.id()),
                        CosmosButton::default(),
                    ))
                    .observe(
                        |on: On<ButtonEvent>, q_category: Query<&SwapToCategory>, mut q_recipes_list: Query<&mut RecipesList>| {
                            // TODO: play sound
                            let Ok(cat) = q_category.get(on.0) else {
                                return;
                            };
                            let Ok(mut recipe_list) = q_recipes_list.single_mut() else {
                                return;
                            };
                            if recipe_list.0 == Some(cat.0) {
                                recipe_list.0 = None;
                            } else {
                                recipe_list.0 = Some(cat.0);
                            }
                        },
                    );
                }
            });

            p.spawn(Node {
                margin: UiRect::all(Val::Px(20.0)),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..Default::default()
            })
            .with_children(|p| {
                // search filter
                p.spawn((
                    Node {
                        width: Val::Auto,
                        margin: UiRect {
                            left: Val::Px(20.0),
                            right: Val::Px(40.0),
                            top: Val::Px(20.0),
                            bottom: Val::Px(20.0),
                        },
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    TextInput { ..Default::default() },
                    BindValues::single(BindValue::<RecipeSearch>::new(root_ent_id, ReactableFields::Value)),
                    BackgroundColor(Srgba::hex("00000033").unwrap().into()),
                    BorderColor::all(css::WHITE),
                    text_style,
                ));

                // craftable items will get populated
                p.spawn((
                    RecipesList(None),
                    Node {
                        flex_grow: 1.0,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,
                        ..Default::default()
                    },
                ));
            });
        });
    }
}

#[derive(Component, Default, Debug)]
struct RecipeCraftState {
    adding: Option<bool>,
    amount: u32,
    seconds_since_last_input_changed: f32,
    last_amount_added: u32,
    last_time_added: f32,
}

fn should_make_another(time: f32, last_time: f32) -> bool {
    fn checker(x: f32) -> f32 {
        4.0_f32.powf(x) - 1.0
    }

    checker(time) - checker(last_time) > 1.0
}

#[derive(Component)]
struct LiveCheckAmount(FabricatorItemInput);

fn update_item_input_text(
    q_changed: Query<(), Or<((With<LocalPlayer>, Changed<Inventory>), Added<LiveCheckAmount>)>>,
    q_inv: Query<&Inventory, With<LocalPlayer>>,
    mut q_live_check: Query<(&mut Text, &LiveCheckAmount, &mut TextColor)>,
) {
    if q_changed.is_empty() {
        return;
    }

    let Ok(inv) = q_inv.single() else {
        return;
    };

    for (mut txt, live_check_amt, mut txt_color) in q_live_check.iter_mut() {
        let in_inv = match live_check_amt.0.item {
            RecipeItem::Item(id) => inv
                .iter()
                .flatten()
                .filter(|x| x.item_id() == id)
                .map(|x| x.quantity() as u32)
                .sum::<u32>(),
        };

        if in_inv >= live_check_amt.0.quantity as u32 {
            txt_color.0 = css::GREEN.into();
        } else {
            txt_color.0 = css::RED.into();
        }
        txt.0 = format!("{}/{}", in_inv, live_check_amt.0.quantity)
    }
}

#[derive(Component)]
struct CraftingDisplay;

#[derive(Component)]
struct CraftingAmountDisplay;

#[derive(Component)]
struct InUseDisplay;

#[derive(Component)]
struct CraftItemBtn;

fn on_add_in_use(
    q_craft_item_btn: Query<Entity, With<CraftItemBtn>>,
    mut rmd_item: RemovedComponents<InUseDisplay>,
    font: Res<DefaultFont>,
    q_added_in_use: Query<(Entity, &Recipe), Added<InUseDisplay>>,
    mut commands: Commands,
) {
    if rmd_item.read().next().is_some() {
        for ent in q_craft_item_btn.iter() {
            commands.entity(ent).despawn();
        }
    }

    for (ent, recipe) in q_added_in_use.iter() {
        commands.entity(ent).with_children(|p| {
            p.spawn((
                Node {
                    padding: UiRect::all(Val::Px(16.0)),
                    height: Val::Px(48.0),
                    ..Default::default()
                },
                Recipe(recipe.0.clone()),
                BackgroundColor(css::GREEN.into()),
                CraftItemBtn,
                CosmosButton {
                    submit_control: Some(CosmosInputs::PerformCraft),
                    text: Some((
                        CHECK.to_string(),
                        TextFont {
                            font: font.get(),
                            font_size: 24.0,
                            ..Default::default()
                        },
                        Default::default(),
                    )),
                    ..Default::default()
                },
            ))
            .observe(
                move |_: On<ButtonEvent>,
                      mut commands: Commands,
                      q_crafting_ui: Query<Entity, With<CraftingDisplay>>,
                      q_craft_state: Query<(&RecipeCraftState, &Recipe)>,
                      mut nevw_craft_event: NettyMessageWriter<CraftBasicFabricatorRecipeMessage>,
                      q_open_fab_menu: Query<&OpenBasicFabMenu>| {
                    let Ok((recipe_state, recipe)) = q_craft_state.get(ent) else {
                        return;
                    };
                    let Ok(fab_menu) = q_open_fab_menu.single() else {
                        return;
                    };

                    nevw_craft_event.write(CraftBasicFabricatorRecipeMessage {
                        block: fab_menu.0,
                        recipe: recipe.0.clone(),
                        quantity: recipe_state.amount,
                    });
                    let Ok(display_ent) = q_crafting_ui.single() else {
                        return;
                    };
                    commands.entity(display_ent).despawn();
                },
            );
        });
    }
}

fn show_recipe_on_hover(
    q_active: Query<(), With<InUseDisplay>>,
    q_craftable_recipes: Query<(Entity, &Recipe, &PickingInteraction), (Without<RecipeCraftState>, Changed<PickingInteraction>)>,
    mut commands: Commands,
    q_shown_recipe_ui: Query<(Entity, &ChildOf), With<RecipeCraftState>>,
    font: Res<DefaultFont>,
    lang: Res<Lang<Item>>,
    items: Res<Registry<Item>>,
) {
    if !q_active.is_empty() {
        return;
    }

    for (ent, recipe, interaction) in q_craftable_recipes.iter() {
        if *interaction == PickingInteraction::None {
            for (shown_ui, shown_recipe) in q_shown_recipe_ui.iter() {
                if shown_recipe.parent() == ent {
                    commands.entity(shown_ui).despawn();
                }
            }
            continue;
        }

        if q_shown_recipe_ui.iter().any(|(_, shown_recipe)| shown_recipe.parent() == ent) {
            continue;
        }

        commands.entity(ent).with_children(|p| {
            p.spawn((
                CraftingDisplay,
                RecipeCraftState::default(),
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(20.0),
                    top: Val::Px(100.0),
                    min_width: Val::Px(300.0),
                    min_height: Val::Px(200.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                GlobalZIndex(2),
                BorderRadius::all(Val::Px(4.0)),
                Recipe(recipe.0.clone()),
                BackgroundColor(Srgba::hex("000000EE").unwrap().into()),
                BorderColor::all(css::AQUA),
                Pickable::default(),
                PickingInteraction::default(),
            ))
            .with_children(|p| {
                let item_name = lang.get_name_or_unlocalized(items.from_numeric_id(recipe.0.output.item));

                p.spawn((
                    Text::new(format!(
                        "{} {}",
                        item_name,
                        if recipe.0.output.quantity != 1 {
                            format!("x{}", recipe.0.output.quantity)
                        } else {
                            "".into()
                        }
                    )),
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    Node {
                        margin: UiRect::right(Val::Px(25.0)),
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Text::new("0"),
                    CraftingAmountDisplay,
                    TextFont {
                        font_size: 24.0,
                        font: font.get(),
                        ..Default::default()
                    },
                    (Node { ..Default::default() }),
                ));

                // p.spawn(Node {
                //     justify_content: JustifyContent::SpaceBetween,
                //     ..Default::default()
                // })
                // .with_children(|p| {
                //     p.spawn((
                //         Text::new(format!(
                //             "{} {}",
                //             item_name,
                //             if recipe.0.output.quantity != 1 {
                //                 format!("x{}", recipe.0.output.quantity)
                //             } else {
                //                 "".into()
                //             }
                //         )),
                //         TextFont {
                //             font_size: 24.0,
                //             font: font.get(),
                //             ..Default::default()
                //         },
                //         Node {
                //             margin: UiRect::right(Val::Px(25.0)),
                //             ..Default::default()
                //         },
                //     ));
                //
                //     p.spawn((
                //         CraftingAmountDisplay,
                //         Text::new(String::new()),
                //         TextFont {
                //             font_size: 24.0,
                //             font: font.get(),
                //             ..Default::default()
                //         },
                //         Node { ..Default::default() },
                //     ));
                // });

                p.spawn(Node::default()).with_children(|p| {
                    for input in recipe.0.inputs.iter() {
                        match input.item {
                            RecipeItem::Item(item_id) => {
                                let mut ecmds = p.spawn((block_item_node(), RenderItem { item_id }));

                                ecmds.with_children(|p| {
                                    p.spawn((
                                        Name::new("Amount Text"),
                                        Node {
                                            bottom: Val::Px(5.0),
                                            right: Val::Px(5.0),
                                            position_type: PositionType::Absolute,

                                            ..Default::default()
                                        },
                                        LiveCheckAmount(input.clone()),
                                        Text::new(""),
                                        TextFont {
                                            font_size: 20.0,
                                            font: font.get(),
                                            ..Default::default()
                                        },
                                        TextLayout {
                                            justify: Justify::Right,
                                            ..Default::default()
                                        },
                                    ));
                                });
                            }
                        }
                    }
                });
            });
        });
    }
}

fn on_press_craftable_item(
    mut commands: Commands,
    q_local_inv: Query<&Inventory, With<LocalPlayer>>,
    q_pointers: Query<&PointerPress>,
    mut q_craftable_recipes: Query<(&Recipe, &mut RecipeCraftState)>,
    inputs: InputChecker,
    time: Res<Time>,
    q_crafting_thing: Query<(Entity, Has<InUseDisplay>), With<CraftingDisplay>>,
    q_hovering_recipe: Query<(&Recipe, &PickingInteraction)>,
    q_craft_btn: Query<&PickingInteraction, With<CraftItemBtn>>,
) {
    let Ok(pointer) = q_pointers.single() else {
        error!("No pointer");
        return;
    };

    let Ok(inventory) = q_local_inv.single() else {
        return;
    };

    if let Ok((recipe, mut state)) = q_craftable_recipes.single_mut() {
        if !pointer.is_primary_pressed() && !pointer.is_secondary_pressed() {
            state.seconds_since_last_input_changed = 0.0;
            state.last_time_added = 0.0;
            state.adding = None;
            return;
        }

        let hovering_craft_btn = q_craft_btn.single().is_ok_and(|x| *x != PickingInteraction::None);
        if hovering_craft_btn {
            return;
        }

        if !q_hovering_recipe
            .iter()
            .any(|(hovering_recipe, interaction)| hovering_recipe.0 == recipe.0 && *interaction != PickingInteraction::None)
        {
            let Ok((ent, _)) = q_crafting_thing.single() else {
                return;
            };

            commands.entity(ent).insert(NeedsDespawned);
            return;
        }

        let prev_state = state.adding;

        let adding = !pointer.is_secondary_pressed();
        state.adding = Some(adding);

        let amt = if inputs.check_pressed(CosmosInputs::Craft100) {
            100
        } else if inputs.check_pressed(CosmosInputs::Craft10) {
            10
        } else {
            1
        } * recipe.0.output.quantity as u32;

        if state.adding != prev_state || state.last_amount_added != amt {
            state.seconds_since_last_input_changed = 0.0;
            state.last_amount_added = amt;
        } else {
            state.seconds_since_last_input_changed += time.delta_secs();
        }

        if state.seconds_since_last_input_changed == 0.0
            || should_make_another(state.seconds_since_last_input_changed, state.last_time_added)
        {
            state.last_time_added = state.seconds_since_last_input_changed;

            if adding {
                let max = recipe.0.max_can_create(inventory.iter().flatten());
                state.amount = max.min(state.amount + amt);
            } else {
                state.amount = state.amount.saturating_sub(amt);
            }
        }

        if let Ok((ent, in_use)) = q_crafting_thing.single() {
            if in_use && state.amount == 0 {
                commands.entity(ent).remove::<InUseDisplay>();
            } else if !in_use && state.amount != 0 {
                commands.entity(ent).insert(InUseDisplay);
            }
        }
    }
}

fn on_change_recipe_state(
    mut q_crafting_amt_display: Query<&mut Text, With<CraftingAmountDisplay>>,
    q_state_change: Query<&RecipeCraftState, Changed<RecipeCraftState>>,
) {
    let Ok(state) = q_state_change.single() else {
        return;
    };

    let Ok(mut txt) = q_crafting_amt_display.single_mut() else {
        return;
    };

    if state.amount == 0 {
        txt.0 = "".into();
    } else {
        txt.0 = format!("{}", state.amount);
    }
}

fn on_change_recipes_list(
    mut commands: Commands,
    q_changed_anything: Query<(), Or<(Changed<RecipesList>, Changed<RecipeSearch>)>>,
    q_search: Query<&RecipeSearch>,
    q_recipes_list: Query<(&RecipesList, Entity)>,
    recipes: Res<BasicFabricatorRecipes>,
    items: Res<Registry<Item>>,
    categories: Res<Registry<ItemCategory>>,
    lang: Res<Lang<Item>>,
) {
    if q_changed_anything.is_empty() {
        return;
    }

    for (list, ent) in q_recipes_list.iter() {
        let selected_cat = list.0.map(|c| categories.from_numeric_id(c));
        let Ok(search) = q_search.single() else {
            return;
        };
        let search_txt = search.0.to_lowercase();
        commands.entity(ent).despawn_children().with_children(|p| {
            let mut filtered_recipes = recipes
                .iter()
                .filter(|recipe| {
                    let item = items.from_numeric_id(recipe.output.item);

                    selected_cat.is_none_or(|c| item.category().is_some_and(|item_c| c.unlocalized_name() == item_c))
                        && (item.unlocalized_name().to_lowercase().contains(&search_txt)
                            || lang.get_name_or_unlocalized(item).to_lowercase().contains(&search_txt))
                })
                .collect::<Vec<_>>();

            filtered_recipes.sort_by_key(|recipe| items.from_numeric_id(recipe.output.item).unlocalized_name());

            for recipe in filtered_recipes {
                p.spawn((
                    NoHoverTooltip,
                    RenderItem {
                        item_id: recipe.output.item,
                    },
                    Pickable::default(),
                    PickingInteraction::default(),
                    Recipe(recipe.clone()),
                    block_item_node(),
                ));
            }
        });
    }
}

fn block_item_node() -> Node {
    Node {
        width: Val::Px(100.0),
        height: Val::Px(100.0),
        margin: UiRect::all(Val::Px(10.0)),
        ..Default::default()
    }
}

#[derive(Component, Default)]
struct FloatingText(f32);

const FLOAT_DIMS: f32 = 96.0;

fn display_crafted_items(mut commands: Commands, mut nevr_craft: MessageReader<BasicFabricatorCraftResultMessage>, font: Res<DefaultFont>) {
    let mut offset = 0.0;
    for ev in nevr_craft.read() {
        commands
            .spawn((
                FloatingText::default(),
                Node {
                    width: Val::Px(FLOAT_DIMS),
                    height: Val::Px(FLOAT_DIMS),
                    left: Val::Px(64.0),
                    top: Val::Px(500.0 + offset),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::End,
                    justify_content: JustifyContent::End,

                    ..Default::default()
                },
                RenderItem { item_id: ev.item_crafted },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Item recipe qty"),
                    Text::new(format!("+ {}", ev.quantity)),
                    TextFont {
                        font: font.get(),
                        font_size: 30.0,
                        ..Default::default()
                    },
                ));
            });

        offset += FLOAT_DIMS;
    }
}

fn move_floating_text(mut commands: Commands, mut q_text: Query<(Entity, &mut Node, &mut FloatingText)>, time: Res<Time>) {
    for (ent, mut node, mut floating_text) in q_text.iter_mut() {
        floating_text.0 += time.delta_secs() * 30.0;
        let Val::Px(v) = node.top else {
            continue;
        };

        let new_v = v - floating_text.0 * time.delta_secs();
        node.top = Val::Px(new_v);

        if new_v < -FLOAT_DIMS {
            commands.entity(ent).despawn();
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<RecipeSearch>(app);

    app.add_systems(
        Update,
        ((
            populate_menu,
            on_change_recipes_list,
            show_recipe_on_hover,
            update_item_input_text,
            on_press_craftable_item,
            on_change_recipe_state,
            move_floating_text,
            display_crafted_items,
            on_add_in_use,
        )
            .chain())
        .chain()
        .in_set(FabricatorMenuSet::PopulateMenu)
        .run_if(in_state(GameState::Playing))
        .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
