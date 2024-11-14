use bevy::{
    app::Update,
    color::{Color, Srgba},
    core::Name,
    prelude::{
        in_state, resource_exists, Added, App, BuildChildren, Commands, Entity, IntoSystemConfigs, NodeBundle, Query, Res, TextBundle,
    },
    text::{Text, TextStyle},
    ui::{FlexDirection, JustifyContent, Style, UiRect, Val},
};
use cosmos_core::{
    crafting::recipes::{basic_fabricator::BasicFabricatorRecipes, RecipeItem},
    item::Item,
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

use crate::{
    lang::Lang,
    ui::{
        components::{
            scollable_container::ScrollBundle,
            window::{GuiWindow, WindowBundle},
        },
        font::DefaultFont,
        item_renderer::RenderItem,
    },
};

use super::{FabricatorMenuSet, OpenBasicFabricatorMenu};

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
                    p.spawn(
                        (NodeBundle {
                            style: Style {
                                height: Val::Px(100.0),
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::SpaceBetween,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    )
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

            p.spawn(
                (NodeBundle {
                    style: Style {
                        height: Val::Px(200.0),
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
            )
            .with_children(|p| {
                p.spawn(TextBundle {
                    text: Text::from_section("Item details", text_style.clone()),
                    ..Default::default()
                });
            });
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        populate_menu
            .in_set(NetworkingSystemsSet::Between)
            .in_set(FabricatorMenuSet::PopulateMenu)
            .run_if(in_state(GameState::Playing))
            .run_if(resource_exists::<BasicFabricatorRecipes>),
    );
}
