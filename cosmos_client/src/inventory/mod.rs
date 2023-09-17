//! Renders the inventory slots and handles all the logic for moving items around

use bevy::{prelude::*, window::PrimaryWindow};
use cosmos_core::{ecs::NeedsDespawned, inventory::Inventory};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    window::setup::CursorFlags,
};

#[derive(Debug, Resource, Clone, Copy, Default)]
enum InventoryState {
    #[default]
    Closed,
    Open,
}

#[derive(Component)]
struct RenderedInventory;

fn toggle_inventory(
    mut inventory_state: ResMut<InventoryState>,
    inputs: Res<CosmosInputHandler>,
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
) {
    if inputs.check_just_pressed(CosmosInputs::ToggleInventory, &keys, &mouse) {
        match *inventory_state {
            InventoryState::Closed => *inventory_state = InventoryState::Open,
            InventoryState::Open => *inventory_state = InventoryState::Closed,
        }
    }
}

#[derive(Component, Debug)]
struct CloseInventoryButton;

fn close_button_system(
    mut inventory_state: ResMut<InventoryState>,
    mut interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>, With<CloseInventoryButton>)>,
) {
    for interaction in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *inventory_state = InventoryState::Closed;
            }
            _ => {}
        }
    }
}

fn toggle_inventory_rendering(
    open_inventory: Query<Entity, With<RenderedInventory>>,
    inventory_state: Res<InventoryState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    local_inventory: Query<&Inventory, With<LocalPlayer>>,
    mut cursor_flags: ResMut<CursorFlags>,
) {
    if !inventory_state.is_changed() {
        return;
    }

    let Ok(local_inventory) = local_inventory.get_single() else {
        warn!("Missing inventory and tried to open it!");
        return;
    };

    match *inventory_state {
        InventoryState::Closed => {
            if let Ok(entity) = open_inventory.get_single() {
                commands.entity(entity).insert(NeedsDespawned);

                cursor_flags.hide();
            }
        }
        InventoryState::Open => {
            cursor_flags.show();

            let font = asset_server.load("fonts/PixeloidSans.ttf");

            commands
                .spawn((
                    Name::new("Rendered Inventory"),
                    RenderedInventory,
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            display: Display::Flex,
                            flex_direction: FlexDirection::Column,
                            left: Val::Px(100.0),
                            top: Val::Px(100.0),
                            width: Val::Px(800.0),
                            height: Val::Px(596.0),
                            border: UiRect::all(Val::Px(2.0)),
                            ..default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    // Title bar
                    parent
                        .spawn((
                            Name::new("Title Bar"),
                            NodeBundle {
                                style: Style {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    width: Val::Percent(100.0),
                                    height: Val::Px(60.0),
                                    padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),

                                    ..default()
                                },
                                background_color: BackgroundColor(Color::WHITE),
                                ..default()
                            },
                            UiImage {
                                texture: asset_server.load("cosmos/images/ui/inventory-header.png"),
                                ..Default::default()
                            },
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle {
                                style: Style { ..default() },
                                text: Text::from_section(
                                    "Inventory",
                                    TextStyle {
                                        color: Color::WHITE,
                                        font_size: 24.0,
                                        font: font.clone(),
                                    },
                                )
                                .with_alignment(TextAlignment::Center),
                                ..default()
                            });

                            parent
                                .spawn((
                                    ButtonBundle {
                                        style: Style {
                                            width: Val::Px(50.0),
                                            height: Val::Px(50.0),
                                            // horizontally center child text
                                            justify_content: JustifyContent::Center,
                                            // vertically center child text
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        background_color: BackgroundColor(Color::WHITE),
                                        image: UiImage {
                                            texture: asset_server.load("cosmos/images/ui/inventory-close-button.png"),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    CloseInventoryButton,
                                ))
                                .with_children(|button| {
                                    button.spawn(TextBundle {
                                        style: Style { ..default() },
                                        text: Text::from_section(
                                            "X",
                                            TextStyle {
                                                color: Color::WHITE,
                                                font_size: 24.0,
                                                font: font.clone(),
                                            },
                                        )
                                        .with_alignment(TextAlignment::Center),
                                        ..default()
                                    });
                                });
                        });

                    parent.spawn((
                        Name::new("Slots"),
                        NodeBundle {
                            style: Style {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                flex_grow: 1.0,
                                // margin: UiRect::new(Val::Px(0.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),
                                ..default()
                            },

                            background_color: BackgroundColor(Color::hex("2D2D2D").unwrap()),
                            ..default()
                        },
                    ));

                    parent.spawn((
                        Name::new("Hotbar Slots"),
                        NodeBundle {
                            style: Style {
                                display: Display::Flex,
                                height: Val::Px(64.0),

                                ..default()
                            },

                            background_color: BackgroundColor(Color::WHITE),
                            ..default()
                        },
                        UiImage {
                            texture: asset_server.load("cosmos/images/ui/inventory-footer.png"),
                            ..Default::default()
                        },
                    ));
                });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (toggle_inventory, close_button_system, toggle_inventory_rendering).chain())
        .init_resource::<InventoryState>();
}
