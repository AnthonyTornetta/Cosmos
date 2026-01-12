use std::cmp::Ordering;

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::data::{BlockData, BlockDataIdentifier},
    crafting::{
        blocks::basic_fabricator::CraftBasicFabricatorRecipeMessage,
        recipes::{
            RecipeItem,
            basic_fabricator::{BasicFabricatorRecipe, BasicFabricatorRecipes},
        },
    },
    inventory::{
        Inventory,
        itemstack::ItemStack,
        netty::{ClientInventoryMessages, InventoryIdentifier},
    },
    item::{Item, item_category::ItemCategory},
    netty::{
        NettyChannelClient,
        client::LocalPlayer,
        cosmos_encoder,
        sync::{
            events::client_event::NettyMessageWriter,
            mapping::{Mappable, NetworkMapping},
        },
    },
    prelude::{Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};
use renet::RenetClient;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed, InventorySide},
    lang::Lang,
    rendering::MainCamera,
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            button::{ButtonEvent, ButtonStyles, CosmosButton},
            scollable_container::ScrollBox,
            show_cursor::ShowCursor,
            text_input::TextInput,
            window::GuiWindow,
        },
        font::DefaultFont,
        item_renderer::{CustomHoverTooltip, RenderItem},
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

use super::{FabricatorMenuSet, OpenBasicFabricatorMenu};

#[derive(Component, Debug, Clone)]
struct Recipe(BasicFabricatorRecipe);

#[derive(Component, Debug)]
struct SelectedRecipe;

#[derive(Component)]
struct FabricateButton;

#[derive(Component, Debug)]
struct DisplayedFabRecipes(StructureBlock);

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
    q_player: Query<Entity, With<LocalPlayer>>,
    font: Res<DefaultFont>,
    crafting_recipes: Res<BasicFabricatorRecipes>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
    q_structure: Query<&Structure>,
    q_inventory: Query<&Inventory>,
    categories: Res<Registry<ItemCategory>>,
    category_lang: Res<Lang<ItemCategory>>,
) {
    for (ent, fab_menu) in q_added_menu.iter() {
        let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
            error!("No structure for basic_fabricator!");
            continue;
        };

        let Some(inventory) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
            error!("No inventory in basic_fabricator!");
            continue;
        };
        let Ok(player_ent) = q_player.single() else {
            return;
        };

        let mut ecmds = commands.entity(ent);

        let text_style = TextFont {
            font: font.0.clone(),
            font_size: 24.0,
            ..Default::default()
        };

        let item_slot_size = 64.0;

        ecmds.insert((
            OpenMenu::new(0),
            BorderRadius::all(Val::Px(20.0)),
            // transparent aqua
            BackgroundColor(Srgba::hex("0099BB99").unwrap().into()),
            BorderColor::all(css::AQUA),
            Node {
                width: Val::Percent(80.0),
                height: Val::Percent(80.0),
                margin: UiRect::AUTO,
                border: UiRect::all(Val::Px(2.0)),
                ..Default::default()
            },
            ShowCursor,
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
                                .map_or(false, |n| n == c.unlocalized_name())
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

            p.spawn(
                (Node {
                    margin: UiRect::all(Val::Px(20.0)),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                }),
            )
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
                            ..Default::default()
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

                // craftable items
                p.spawn((
                    RecipesList(None),
                    Node {
                        flex_grow: 1.0,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {});
            });
        });
    }
}

fn create_ui_recipes_list(
    crafting_recipes: &BasicFabricatorRecipes,
    items: &Registry<Item>,
    lang: &Lang<Item>,
    text_style: &TextFont,
    inv_items: Vec<&ItemStack>,
    p: &mut ChildSpawnerCommands,
    selected: Option<&Recipe>,
) {
    let mut recipes = crafting_recipes.iter().collect::<Vec<_>>();
    recipes.sort_by(|a, b| {
        // Sort by craftable then by name.

        let a_create = a.max_can_create(inv_items.iter().copied());
        let b_create = b.max_can_create(inv_items.iter().copied());

        let amount_diff = a
            .inputs
            .iter()
            .filter(|x| match x.item {
                RecipeItem::Item(i) => inv_items.iter().any(|x| x.item_id() == i),
            })
            .count() as i32
            - b.inputs
                .iter()
                .filter(|x| match x.item {
                    RecipeItem::Item(i) => inv_items.iter().any(|x| x.item_id() == i),
                })
                .count() as i32;

        if a_create == 0 && b_create != 0 {
            Ordering::Greater
        } else if a_create != 0 && b_create == 0 {
            Ordering::Less
        } else if a_create != 0 && b_create != 0 && a.inputs.len() != b.inputs.len() {
            b.inputs.len().cmp(&a.inputs.len())
        } else if amount_diff > 0 {
            Ordering::Less
        } else if amount_diff < 0 {
            Ordering::Greater
        } else {
            let a_name = lang
                .get_name_from_numeric_id(a.output.item)
                .unwrap_or(items.from_numeric_id(a.output.item).unlocalized_name());
            let b_name = lang
                .get_name_from_numeric_id(b.output.item)
                .unwrap_or(items.from_numeric_id(b.output.item).unlocalized_name());

            a_name.to_lowercase().cmp(&b_name.to_lowercase())
        }
    });

    for &recipe in recipes.iter() {
        let mut ecmds = p.spawn((
            Node {
                height: Val::Px(100.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            CosmosButton::default(),
            Recipe(recipe.clone()),
        ));

        ecmds.observe(on_select_item);

        ecmds.with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    margin: UiRect::all(Val::Auto),
                    ..Default::default()
                },
                RenderItem {
                    item_id: recipe.output.item,
                },
            ));

            let item = items.from_numeric_id(recipe.output.item);
            let name = lang.get_name_from_id(item.unlocalized_name()).unwrap_or(item.unlocalized_name());

            p.spawn((
                Name::new("Item name + inputs display"),
                Node {
                    width: Val::Percent(80.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        // margin: UiRect::vertical(Val::Auto),
                        ..Default::default()
                    },
                    Text::new(format!("{}x {}", recipe.output.quantity, name)),
                    text_style.clone(),
                ));

                p.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    width: Val::Percent(100.0),
                    ..Default::default()
                })
                .with_children(|p| {
                    for item in recipe.inputs.iter() {
                        let RecipeItem::Item(item_id) = item.item;

                        p.spawn((
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::End,
                                justify_content: JustifyContent::End,
                                ..Default::default()
                            },
                            RenderItem { item_id },
                        ))
                        .with_children(|p| {
                            p.spawn((
                                Name::new("Item recipe qty"),
                                Text::new(format!("{}", item.quantity)),
                                text_style.clone(),
                            ));
                        });
                    }
                });
            });
        });

        if let Some(s) = selected
            && recipe == &s.0
        {
            ecmds.insert((
                SelectedRecipe,
                BackgroundColor(
                    Srgba {
                        red: 1.0,
                        green: 1.0,
                        blue: 1.0,
                        alpha: 0.1,
                    }
                    .into(),
                ),
            ));
        }
    }
}

fn on_change_inventory(
    q_changed_inventory: Query<(&Inventory, &BlockData), Changed<Inventory>>,
    q_fab_recipes: Query<(Entity, &DisplayedFabRecipes)>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipe>>,
    crafting_recipes: Res<BasicFabricatorRecipes>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
    font: Res<DefaultFont>,
    mut commands: Commands,
) {
    for (inv, bd) in q_changed_inventory.iter() {
        for (ent, fab_recipes) in q_fab_recipes.iter() {
            if fab_recipes.0 != bd.identifier.block {
                continue;
            }

            let selected = q_selected_recipe.single().ok();

            let text_style = TextFont {
                font: font.0.clone(),
                font_size: 16.0,
                ..Default::default()
            };

            let inv_items = inv.iter().flatten().collect::<Vec<_>>();

            commands.entity(ent).despawn_related::<Children>().with_children(|p| {
                create_ui_recipes_list(&crafting_recipes, &items, &lang, &text_style, inv_items, p, selected);
            });
        }
    }
}

fn auto_insert_items(
    recipe: &BasicFabricatorRecipe,
    player_inv: &Inventory,
    fab_inv: &Inventory,
    client: &mut RenetClient,
    mapping: &NetworkMapping,
    fab_inv_block: StructureBlock,
    fab_block_id: u16,
    player_inv_ent: Entity,
) {
    let Ok(fab_inv_block) = fab_inv_block.map_to_server(mapping) else {
        return;
    };
    let Some(player_inv_ent) = mapping.server_from_client(&player_inv_ent) else {
        return;
    };

    for (needed_id, mut already_there) in recipe.inputs.iter().filter_map(|x| {
        let RecipeItem::Item(id) = x.item;
        let already_there = fab_inv
            .iter()
            .flatten()
            .filter(|item| item.item_id() == id)
            .map(|x| x.quantity())
            .sum::<u16>();

        if already_there < x.quantity {
            Some((id, already_there))
        } else {
            None
        }
    }) {
        for (slot, is) in player_inv
            .iter()
            .enumerate()
            .flat_map(|(i, x)| x.as_ref().map(|x| (i, x)))
            .filter(|(_, x)| x.item_id() == needed_id)
        {
            let max_amt = is.max_stack_size() - already_there;
            let take_amt = is.quantity().min(max_amt);

            already_there += take_amt;

            if take_amt != 0 {
                client.send_message(
                    NettyChannelClient::Inventory,
                    cosmos_encoder::serialize(&ClientInventoryMessages::AutoMove {
                        from_slot: slot as u32,
                        quantity: take_amt,
                        from_inventory: InventoryIdentifier::Entity(player_inv_ent),
                        to_inventory: InventoryIdentifier::BlockData(BlockDataIdentifier {
                            block: fab_inv_block,
                            block_id: fab_block_id,
                        }),
                    }),
                );
            }

            if take_amt < is.quantity() {
                break;
            }
        }
    }
}

fn on_select_item(
    ev: On<ButtonEvent>,
    mut commands: Commands,
    q_selected_recipe: Query<Entity, With<SelectedRecipe>>,
    q_recipe: Query<&Recipe>,
    q_menu: Query<&DisplayedFabRecipes>,
    mut q_bg_col: Query<&mut BackgroundColor>,
    q_player: Query<(Entity, &Inventory), With<LocalPlayer>>,
    q_structure: Query<&Structure>,
    q_inventory: Query<&Inventory>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
) {
    if let Ok(selected_recipe) = q_selected_recipe.single() {
        if ev.0 == selected_recipe {
            let Ok(recipe) = q_recipe.get(selected_recipe) else {
                return;
            };

            let Ok((player_ent, player_inv)) = q_player.single() else {
                return;
            };

            let Ok(menu) = q_menu.single() else {
                return;
            };

            let Ok(structure) = q_structure.get(menu.0.structure()) else {
                return;
            };

            let Some(inv) = structure.query_block_data(menu.0.coords(), &q_inventory) else {
                return;
            };

            auto_insert_items(
                &recipe.0,
                player_inv,
                inv,
                &mut client,
                &mapping,
                menu.0,
                structure.block_id_at(menu.0.coords()),
                player_ent,
            );
            return;
        }
        commands.entity(selected_recipe).remove::<SelectedRecipe>();
        q_bg_col.get_mut(selected_recipe).expect("Must be ui node").0 = Color::NONE;
    }
    commands.entity(ev.0).insert(SelectedRecipe);
    q_bg_col.get_mut(ev.0).expect("Must be ui node").0 = Srgba {
        red: 1.0,
        green: 1.0,
        blue: 1.0,
        alpha: 0.1,
    }
    .into();
}

fn listen_create(
    _trigger: On<ButtonEvent>,
    q_structure: Query<&Structure>,
    q_inventory: Query<&Inventory>,
    q_open_fab_menu: Query<&OpenBasicFabricatorMenu>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipe>>,
    mut nevw_craft_event: NettyMessageWriter<CraftBasicFabricatorRecipeMessage>,
    network_mapping: Res<NetworkMapping>,
    input_handler: InputChecker,
) {
    let Ok(fab_menu) = q_open_fab_menu.single() else {
        return;
    };

    let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
        return;
    };
    let Some(block_inv) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
        return;
    };

    let Ok(recipe) = q_selected_recipe.single() else {
        return;
    };

    let max_can_create = recipe.0.max_can_create(block_inv.iter().flatten());

    if max_can_create == 0 {
        return;
    }

    if let Ok(block) = fab_menu.0.map_to_server(&network_mapping) {
        let quantity = if input_handler.check_pressed(CosmosInputs::BulkCraft) {
            max_can_create
        } else {
            recipe.0.output.quantity as u32
        };

        info!("Sending craft {quantity} event!");

        nevw_craft_event.write(CraftBasicFabricatorRecipeMessage {
            block,
            recipe: recipe.0.clone(),
            quantity,
        });
    }
}

fn color_fabricate_button(
    q_open_fab_menu: Query<&OpenBasicFabricatorMenu>,
    q_structure: Query<&Structure>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipe>>,
    q_inventory: Query<&Inventory>,
    mut q_fab_button: Query<&mut CosmosButton, With<FabricateButton>>,
) {
    let Ok(mut btn) = q_fab_button.single_mut() else {
        return;
    };

    let Ok(fab_menu) = q_open_fab_menu.single() else {
        return;
    };

    let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
        return;
    };

    let Some(inventory) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
        return;
    };

    let Ok(recipe) = q_selected_recipe.single() else {
        return;
    };

    if recipe.0.max_can_create(inventory.iter().flatten()) == 0 {
        btn.button_styles = Some(ButtonStyles::default());
    } else {
        btn.button_styles = Some(ButtonStyles {
            background_color: css::GREEN.into(),
            foreground_color: css::WHITE.into(),
            hover_background_color: css::GREEN.into(),
            hover_foreground_color: css::WHITE.into(),
            press_background_color: css::GREEN.into(),
            press_foreground_color: css::WHITE.into(),
        });
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

                    selected_cat.map_or(true, |c| item.category().map_or(false, |item_c| c.unlocalized_name() == item_c))
                        && (item.unlocalized_name().to_lowercase().contains(&search_txt)
                            || lang.get_name_or_unlocalized(item).to_lowercase().contains(&search_txt))
                })
                .collect::<Vec<_>>();

            filtered_recipes.sort_by_key(|recipe| items.from_numeric_id(recipe.output.item).unlocalized_name());

            for recipe in filtered_recipes {
                p.spawn((
                    RenderItem {
                        item_id: recipe.output.item,
                    },
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

pub(super) fn register(app: &mut App) {
    add_reactable_type::<RecipeSearch>(app);

    app.add_systems(
        Update,
        ((populate_menu, on_change_recipes_list).chain())
            .chain()
            .in_set(FabricatorMenuSet::PopulateMenu)
            .run_if(in_state(GameState::Playing))
            .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
