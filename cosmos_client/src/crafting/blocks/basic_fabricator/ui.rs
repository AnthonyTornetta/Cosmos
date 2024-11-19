use bevy::{
    app::Update,
    color::{palettes::css, Color, Srgba},
    core::Name,
    prelude::{
        in_state, resource_exists, Added, App, BuildChildren, Changed, Commands, Component, DespawnRecursiveExt, Entity, Event,
        EventReader, IntoSystemConfigs, NodeBundle, Parent, Query, Res, TextBundle, With,
    },
    text::{Text, TextStyle},
    ui::{FlexDirection, JustifyContent, Style, UiRect, Val},
};
use cosmos_core::{
    crafting::recipes::{
        basic_fabricator::{BasicFabricatorRecipe, BasicFabricatorRecipes},
        RecipeItem,
    },
    inventory::Inventory,
    item::Item,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

use crate::{
    lang::Lang,
    ui::{
        components::{
            button::{register_button, Button, ButtonBundle, ButtonEvent},
            scollable_container::ScrollBundle,
            window::{GuiWindow, WindowBundle},
        },
        font::DefaultFont,
        item_renderer::RenderItem,
        UiSystemSet,
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

#[derive(Component, Debug)]
struct Recipe(BasicFabricatorRecipe);

fn populate_menu(
    mut commands: Commands,
    q_added_menu: Query<Entity, Added<OpenBasicFabricatorMenu>>,
    font: Res<DefaultFont>,
    crafting_recipes: Res<BasicFabricatorRecipes>,
    items: Res<Registry<Item>>,
    lang: Res<Lang<Item>>,
) {
    for ent in q_added_menu.iter() {
        let mut ecmds = commands.entity(ent);

        let text_style = TextStyle {
            font: font.0.clone_weak(),
            font_size: 24.0,
            color: Color::WHITE,
        };

        ecmds.insert(
            (WindowBundle {
                node_bundle: NodeBundle {
                    background_color: Srgba::hex("2D2D2D").unwrap().into(),
                    style: Style {
                        width: Val::Px(400.0),
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
            }),
        );

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
                SelectedRecipeDisplay,
                NodeBundle {
                    style: Style {
                        height: Val::Px(200.0),
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));
        });
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
    q_selected_recipe_display: Query<(Entity, &Parent), With<SelectedRecipeDisplay>>,
    q_inventory: Query<&Inventory, With<LocalPlayer>>,
    mut evr_select_item: EventReader<SelectItemEvent>,
    q_recipe: Query<&Recipe>,
    font: Res<DefaultFont>,
) {
    for ev in evr_select_item.read() {
        let Ok(recipe) = q_recipe.get(ev.0) else {
            continue;
        };

        let Ok((selected_recipe_display, parent)) = q_selected_recipe_display.get_single() else {
            return;
        };

        commands.entity(selected_recipe_display).despawn_recursive();

        let text_style_enough = TextStyle {
            font: font.0.clone_weak(),
            font_size: 24.0,
            color: Color::WHITE,
        };
        let text_style_not_enough = TextStyle {
            font: font.0.clone_weak(),
            font_size: 24.0,
            color: css::RED.into(),
        };

        let Ok(inventory) = q_inventory.get_single() else {
            continue;
        };

        commands
            .spawn((
                SelectedRecipeDisplay,
                NodeBundle {
                    style: Style {
                        height: Val::Px(200.0),
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|p| {
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
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        RenderItem { item_id },
                    ));

                    let inventory_count = inventory.total_quantity_of_item(item_id);

                    let text_style = if inventory_count >= item.quantity as u64 {
                        text_style_enough.clone()
                    } else {
                        text_style_not_enough.clone()
                    };

                    p.spawn((
                        Name::new("Item recipe qty"),
                        InventoryCount {
                            item_id,
                            recipe_amt: item.quantity,
                        },
                        TextBundle {
                            text: Text::from_section(format!("{}/{}", inventory_count, item.quantity), text_style.clone()),
                            ..Default::default()
                        },
                    ));
                }
            })
            .set_parent(parent.get());

        commands
            .spawn((
                bevy::prelude::ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Px(100.0),
                        ..Default::default()
                    },
                    background_color: css::AQUA.into(),

                    ..Default::default()
                },
                Name::new("Craft button"),
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Fabricate Button"),
                    TextBundle {
                        text: Text::from_section("FABRICATE", text_style_enough),
                        ..Default::default()
                    },
                ));
            })
            .set_parent(parent.get());

        println!("{recipe:?}");
    }
}

fn update_inventory_counts(
    font: Res<DefaultFont>,
    mut q_inventory_counts: Query<(&mut Text, &InventoryCount)>,
    q_changed_inventory: Query<&Inventory, (With<LocalPlayer>, Changed<Inventory>)>,
) {
    let Ok(inventory) = q_changed_inventory.get_single() else {
        return;
    };

    let text_style_enough = TextStyle {
        font: font.0.clone_weak(),
        font_size: 24.0,
        color: Color::WHITE,
    };
    let text_style_not_enough = TextStyle {
        font: font.0.clone_weak(),
        font_size: 24.0,
        color: css::RED.into(),
    };

    for (mut text, recipe_info) in q_inventory_counts.iter_mut() {
        let inventory_count = inventory.total_quantity_of_item(recipe_info.item_id);

        let text_style = if inventory_count >= recipe_info.recipe_amt as u64 {
            text_style_enough.clone()
        } else {
            text_style_not_enough.clone()
        };

        text.sections[0].style = text_style;
        text.sections[0].value = format!("{}/{}", inventory_count, recipe_info.recipe_amt);
    }
}

pub(super) fn register(app: &mut App) {
    register_button::<SelectItemEvent>(app);

    app.add_systems(
        Update,
        (
            populate_menu,
            (on_select_item, update_inventory_counts).chain().in_set(UiSystemSet::DoUi),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(FabricatorMenuSet::PopulateMenu)
            .run_if(in_state(GameState::Playing))
            .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
