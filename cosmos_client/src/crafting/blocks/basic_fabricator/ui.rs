use bevy::{
    app::Update,
    color::{palettes::css, Color, Srgba},
    core::Name,
    log::error,
    prelude::{
        in_state, resource_exists, Added, App, BuildChildren, Changed, Children, Commands, Component, DespawnRecursiveExt, Entity, Event,
        EventReader, IntoSystemConfigs, NodeBundle, Parent, Query, Res, TextBundle, With,
    },
    text::{Text, TextStyle},
    ui::{AlignItems, FlexDirection, JustifyContent, Style, TargetCamera, UiRect, Val},
};
use cosmos_core::{
    crafting::recipes::{
        basic_fabricator::{BasicFabricatorRecipe, BasicFabricatorRecipes},
        RecipeItem,
    },
    ecs::NeedsDespawned,
    inventory::Inventory,
    item::Item,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    prelude::Structure,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

use crate::{
    inventory::{CustomInventoryRender, InventoryNeedsDisplayed, InventorySide, TextNeedsTopRoot},
    lang::Lang,
    rendering::MainCamera,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent, ButtonStyles},
            scollable_container::ScrollBundle,
            window::{GuiWindow, WindowBundle},
        },
        font::DefaultFont,
        item_renderer::RenderItem,
        OpenMenu, UiSystemSet,
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

        let text_style = TextStyle {
            font: font.0.clone_weak(),
            font_size: 24.0,
            color: Color::WHITE,
        };

        let item_slot_size = 64.0;

        ecmds.insert((
            TargetCamera(cam),
            OpenMenu::new(0),
            WindowBundle {
                node_bundle: NodeBundle {
                    background_color: Srgba::hex("2D2D2D").unwrap().into(),
                    style: Style {
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
                    ..Default::default()
                },
                window: GuiWindow {
                    title: "Basic Fabricator".into(),
                    body_styles: Style {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        ));

        let mut slot_ents = vec![];

        ecmds.with_children(|p| {
            p.spawn(
                (ScrollBundle {
                    node_bundle: NodeBundle {
                        style: Style {
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .with_children(|p| {
                for recipe in crafting_recipes.iter() {
                    p.spawn((
                        ButtonBundle {
                            node_bundle: NodeBundle {
                                style: Style {
                                    height: Val::Px(100.0),
                                    width: Val::Percent(100.0),
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            button: Button::<SelectItemEvent>::default(),
                        },
                        Recipe(recipe.clone()),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    margin: UiRect::all(Val::Auto),
                                    ..Default::default()
                                },
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
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(80.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::SpaceEvenly,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .with_children(|p| {
                            p.spawn(TextBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    // margin: UiRect::vertical(Val::Auto),
                                    ..Default::default()
                                },
                                text: Text::from_section(format!("{}x {}", recipe.output.quantity, name), text_style.clone()),
                                ..Default::default()
                            });

                            p.spawn(
                                (NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Row,
                                        width: Val::Percent(100.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                            )
                            .with_children(|p| {
                                for item in recipe.inputs.iter() {
                                    p.spawn((
                                        NodeBundle {
                                            style: Style {
                                                width: Val::Px(64.0),
                                                height: Val::Px(64.0),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                        RenderItem {
                                            item_id: match item.item {
                                                RecipeItem::Item(i) => i,
                                                RecipeItem::Category(_) => todo!("Categories"),
                                            },
                                        },
                                    ));
                                }
                            });
                        });
                    });
                }
            });

            p.spawn((
                Name::new("Footer"),
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        height: Val::Px(item_slot_size * 3.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    SelectedRecipeDisplay,
                    Name::new("Selected Recipe Display"),
                    NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            width: Val::Px(item_slot_size * 6.0),
                            height: Val::Px(item_slot_size),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Name::new("Rendered Inventory"),
                    NodeBundle {
                        style: Style {
                            width: Val::Px(item_slot_size * 6.0),
                            height: Val::Px(item_slot_size),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    for (slot, _) in inventory.iter().enumerate() {
                        let ent = p
                            .spawn((
                                Name::new("Rendered Item"),
                                NodeBundle {
                                    style: Style {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                            ))
                            .id();
                        slot_ents.push((slot, ent));
                    }
                });

                p.spawn((
                    Name::new("Fabricate Button"),
                    ButtonBundle {
                        button: Button::<CreateClickedEvent> {
                            text: Some(("Fabricate".into(), text_style)),
                            button_styles: Some(ButtonStyles { ..Default::default() }),
                            ..Default::default()
                        },
                        node_bundle: NodeBundle {
                            style: Style {
                                flex_grow: 1.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    },
                ));
            });
        });

        commands
            .entity(structure.block_data(fab_menu.0.coords()).expect("Must expect from above"))
            .insert(InventoryNeedsDisplayed::Custom(CustomInventoryRender::new(slot_ents)));
    }
}

#[derive(Debug, Component)]
struct SelectedRecipeDisplay;

#[derive(Component, Debug)]
struct InventoryCount {
    recipe_amt: u16,
    item_id: u16,
}

fn on_select_item(
    mut commands: Commands,
    q_selected_recipe_display: Query<(Entity, Option<&Children>), With<SelectedRecipeDisplay>>,
    mut evr_select_item: EventReader<SelectItemEvent>,
    q_recipe: Query<&Recipe>,
    font: Res<DefaultFont>,
) {
    for ev in evr_select_item.read() {
        let Ok(recipe) = q_recipe.get(ev.0) else {
            continue;
        };

        let Ok((selected_recipe_display, children)) = q_selected_recipe_display.get_single() else {
            return;
        };

        if let Some(children) = children {
            for &child in children.iter() {
                commands.entity(child).despawn_recursive();
            }
        }

        let text_style = TextStyle {
            font: font.0.clone_weak(),
            font_size: 24.0,
            color: Color::WHITE,
        };
        // let text_style_not_enough = TextStyle {
        //     font: font.0.clone_weak(),
        //     font_size: 24.0,
        //     color: css::RED.into(),
        // };
        //
        // let Ok(inventory) = q_inventory.get_single() else {
        //     continue;
        // };

        commands.entity(selected_recipe_display).insert(recipe.clone()).with_children(|p| {
            for item in recipe.0.inputs.iter() {
                let item_id = match item.item {
                    RecipeItem::Item(i) => i,
                    RecipeItem::Category(_) => todo!("Categories"),
                };

                p.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::End,
                            justify_content: JustifyContent::End,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    RenderItem { item_id },
                ))
                .with_children(|p| {
                    p.spawn((
                        Name::new("Item recipe qty"),
                        TextNeedsTopRoot,
                        InventoryCount {
                            item_id,
                            recipe_amt: item.quantity,
                        },
                        TextBundle {
                            text: Text::from_section(format!("{}", item.quantity), text_style.clone()),
                            ..Default::default()
                        },
                    ));
                });
            }
        });

        println!("{recipe:?}");
    }
}

fn listen_create(
    q_structure: Query<&Structure>,
    q_inventory: Query<&Inventory>,
    q_open_fab_menu: Query<&OpenBasicFabricatorMenu>,
    q_selected_recipe: Query<&Recipe, With<SelectedRecipeDisplay>>,
    mut evr_create: EventReader<CreateClickedEvent>,
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

        println!("Create {max_can_create}!");
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<SelectItemEvent>(app);
    register_button::<CreateClickedEvent>(app);

    app.add_systems(
        Update,
        (populate_menu, (on_select_item, listen_create).chain().in_set(UiSystemSet::DoUi))
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(FabricatorMenuSet::PopulateMenu)
            .run_if(in_state(GameState::Playing))
            .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
