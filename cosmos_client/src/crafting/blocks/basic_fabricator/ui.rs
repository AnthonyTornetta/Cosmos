use std::cmp::Ordering;

use bevy::{
    app::Update,
    color::{Color, Srgba, palettes::css},
    core::Name,
    log::{error, info},
    prelude::{
        Added, App, BuildChildren, Changed, ChildBuild, ChildBuilder, Commands, Component, DespawnRecursiveExt, Entity, Event, EventReader,
        IntoSystemConfigs, Query, Res, ResMut, Text, With, in_state, resource_exists,
    },
    text::TextFont,
    ui::{AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node, TargetCamera, UiRect, Val},
};
use cosmos_core::{
    block::data::{BlockData, BlockDataIdentifier},
    crafting::{
        blocks::basic_fabricator::CraftBasicFabricatorRecipeEvent,
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
    item::Item,
    netty::{
        NettyChannelClient,
        client::LocalPlayer,
        cosmos_encoder,
        sync::{
            events::client_event::NettyEventWriter,
            mapping::{Mappable, NetworkMapping},
        },
        system_sets::NetworkingSystemsSet,
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
            button::{ButtonEvent, ButtonStyles, CosmosButton, register_button},
            scollable_container::ScrollBox,
            window::GuiWindow,
        },
        font::DefaultFont,
        item_renderer::RenderItem,
    },
};

use super::{FabricatorMenuSet, OpenBasicFabricatorMenu};

#[derive(Event, Debug)]
struct SelectItemEvent(Entity);
impl ButtonEvent for SelectItemEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

#[derive(Event, Debug)]
struct CreateClickedEvent;
impl ButtonEvent for CreateClickedEvent {
    fn create_event(_: Entity) -> Self {
        Self
    }
}

#[derive(Component, Debug, Clone)]
struct Recipe(BasicFabricatorRecipe);

#[derive(Component, Debug)]
struct SelectedRecipe;

#[derive(Component)]
struct FabricateButton;

#[derive(Component, Debug)]
struct DisplayedFabRecipes(StructureBlock);

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
    q_cam: Query<Entity, With<MainCamera>>,
) {
    for (ent, fab_menu) in q_added_menu.iter() {
        let Ok(cam) = q_cam.get_single() else {
            return;
        };

        let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
            error!("No structure for basic_fabricator!");
            continue;
        };

        let Some(inventory) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
            error!("No inventory in basic_fabricator!");
            continue;
        };
        let Ok(player_ent) = q_player.get_single() else {
            return;
        };

        commands
            .entity(player_ent)
            .insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));

        let mut ecmds = commands.entity(ent);

        let text_style = TextFont {
            font: font.0.clone_weak(),
            font_size: 24.0,
            ..Default::default()
        };

        let item_slot_size = 64.0;

        ecmds.insert((
            TargetCamera(cam),
            OpenMenu::new(0),
            BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
            Node {
                width: Val::Px(item_slot_size * 6.0),
                height: Val::Px(800.0),
                margin: UiRect {
                    // Centers it vertically
                    top: Val::Auto,
                    bottom: Val::Auto,
                    // Aligns it 100px from the right
                    left: Val::Auto,
                    right: Val::Px(100.0),
                },
                ..Default::default()
            },
            GuiWindow {
                title: "Basic Fabricator".into(),
                body_styles: Node {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            },
        ));

        let mut slot_ents = vec![];

        let inv_items = inventory.iter().flatten().collect::<Vec<_>>();

        ecmds.with_children(|p| {
            p.spawn((
                ScrollBox::default(),
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    DisplayedFabRecipes(fab_menu.0),
                ))
                .with_children(|p| {
                    create_ui_recipes_list(&crafting_recipes, &items, &lang, &text_style, inv_items, p, None);
                });
            });

            p.spawn((
                Name::new("Footer"),
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    height: Val::Px(item_slot_size * 2.0),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Rendered Inventory"),
                    Node {
                        width: Val::Px(item_slot_size * 6.0),
                        height: Val::Px(item_slot_size),
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    for (slot, _) in inventory.iter().enumerate() {
                        let ent = p
                            .spawn((
                                Name::new("Rendered Item"),
                                Node {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    ..Default::default()
                                },
                            ))
                            .id();
                        slot_ents.push((slot, ent));
                    }
                });

                p.spawn((
                    Name::new("Fabricate Button"),
                    FabricateButton,
                    CosmosButton::<CreateClickedEvent> {
                        text: Some(("Fabricate".into(), text_style, Default::default())),
                        button_styles: Some(ButtonStyles { ..Default::default() }),
                        ..Default::default()
                    },
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                ));
            });
        });

        commands
            .entity(structure.block_data(fab_menu.0.coords()).expect("Must expect from above"))
            .insert(InventoryNeedsDisplayed::Custom(CustomInventoryRender::new(slot_ents)));
    }
}

fn create_ui_recipes_list(
    crafting_recipes: &BasicFabricatorRecipes,
    items: &Registry<Item>,
    lang: &Lang<Item>,
    text_style: &TextFont,
    inv_items: Vec<&ItemStack>,
    p: &mut ChildBuilder,
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
            CosmosButton::<SelectItemEvent>::default(),
            Recipe(recipe.clone()),
        ));

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

        if let Some(s) = selected {
            if recipe == &s.0 {
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

            let selected = q_selected_recipe.get_single().ok();

            let text_style = TextFont {
                font: font.0.clone_weak(),
                font_size: 24.0,
                ..Default::default()
            };

            let inv_items = inv.iter().flatten().collect::<Vec<_>>();

            commands.entity(ent).despawn_descendants().with_children(|p| {
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
    let Ok(fab_inv_block) = fab_inv_block.map_to_server(&mapping) else {
        return;
    };
    let Some(player_inv_ent) = mapping.server_from_client(&player_inv_ent) else {
        return;
    };

    for (needed_id, mut already_there) in recipe.inputs.iter().filter_map(|x| {
        let id = match x.item {
            RecipeItem::Item(i) => i,
        };
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
    mut commands: Commands,
    mut evr_select_item: EventReader<SelectItemEvent>,
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
    for ev in evr_select_item.read() {
        if let Ok(selected_recipe) = q_selected_recipe.get_single() {
            if ev.0 == selected_recipe {
                let Ok(recipe) = q_recipe.get(selected_recipe) else {
                    continue;
                };

                let Ok((player_ent, player_inv)) = q_player.get_single() else {
                    continue;
                };

                let Ok(menu) = q_menu.get_single() else {
                    continue;
                };

                let Ok(structure) = q_structure.get(menu.0.structure()) else {
                    continue;
                };

                let Some(inv) = structure.query_block_data(menu.0.coords(), &q_inventory) else {
                    continue;
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
                continue;
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
}

fn listen_create(
    q_structure: Query<&Structure>,
    q_inventory: Query<&Inventory>,
    q_open_fab_menu: Query<&OpenBasicFabricatorMenu>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipe>>,
    mut evr_create: EventReader<CreateClickedEvent>,
    mut nevw_craft_event: NettyEventWriter<CraftBasicFabricatorRecipeEvent>,
    network_mapping: Res<NetworkMapping>,
    input_handler: InputChecker,
) {
    for _ in evr_create.read() {
        let Ok(fab_menu) = q_open_fab_menu.get_single() else {
            return;
        };

        let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
            continue;
        };
        let Some(block_inv) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
            continue;
        };

        let Ok(recipe) = q_selected_recipe.get_single() else {
            return;
        };

        let max_can_create = recipe.0.max_can_create(block_inv.iter().flatten());

        if max_can_create == 0 {
            continue;
        }

        if let Ok(block) = fab_menu.0.map_to_server(&network_mapping) {
            let quantity = if input_handler.check_pressed(CosmosInputs::BulkCraft) {
                max_can_create
            } else {
                recipe.0.output.quantity as u32
            };

            info!("Sending craft {quantity} event!");

            nevw_craft_event.send(CraftBasicFabricatorRecipeEvent {
                block,
                recipe: recipe.0.clone(),
                quantity,
            });
        }
    }
}

fn color_fabricate_button(
    q_open_fab_menu: Query<&OpenBasicFabricatorMenu>,
    q_structure: Query<&Structure>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipe>>,
    q_inventory: Query<&Inventory>,
    mut q_fab_button: Query<&mut CosmosButton<CreateClickedEvent>, With<FabricateButton>>,
) {
    let Ok(mut btn) = q_fab_button.get_single_mut() else {
        return;
    };

    let Ok(fab_menu) = q_open_fab_menu.get_single() else {
        return;
    };

    let Ok(structure) = q_structure.get(fab_menu.0.structure()) else {
        return;
    };

    let Some(inventory) = structure.query_block_data(fab_menu.0.coords(), &q_inventory) else {
        return;
    };

    let Ok(recipe) = q_selected_recipe.get_single() else {
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

pub(super) fn register(app: &mut App) {
    register_button::<SelectItemEvent>(app);
    register_button::<CreateClickedEvent>(app);

    app.add_systems(
        Update,
        (
            (populate_menu, on_change_inventory).chain(),
            (on_select_item, listen_create, color_fabricate_button)
                .chain()
                .in_set(UiSystemSet::DoUi),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(FabricatorMenuSet::PopulateMenu)
            .run_if(in_state(GameState::Playing))
            .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
