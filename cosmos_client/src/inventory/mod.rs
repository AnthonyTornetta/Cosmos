//! Renders the inventory slots and handles all the logic for moving items around

use bevy::{ecs::system::EntityCommands, prelude::*};
use cosmos_core::{
    ecs::NeedsDespawned,
    inventory::{itemstack::ItemStack, Inventory},
};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    ui::item_renderer::RenderItem,
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
    local_inventory: Query<(Entity, &Inventory), With<LocalPlayer>>,
    mut cursor_flags: ResMut<CursorFlags>,
) {
    if !inventory_state.is_changed() {
        return;
    }

    let Ok((inventory_holder, local_inventory)) = local_inventory.get_single() else {
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

            let text_style = TextStyle {
                color: Color::WHITE,
                font_size: 22.0,
                font: font.clone(),
            };

            let inventory_border_size = 2.0;
            let n_slots_per_row: usize = 9;
            let slot_size = 64.0;

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
                            width: Val::Px(n_slots_per_row as f32 * slot_size + inventory_border_size * 2.0),
                            border: UiRect::all(Val::Px(inventory_border_size)),
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

                    parent
                        .spawn((
                            Name::new("Non-Hotbar Slots"),
                            NodeBundle {
                                style: Style {
                                    display: Display::Grid,
                                    flex_grow: 1.0,
                                    grid_column: GridPlacement::end(n_slots_per_row as i16),
                                    grid_template_columns: vec![RepeatedGridTrack::px(
                                        GridTrackRepetition::Count(n_slots_per_row as u16),
                                        slot_size,
                                    )],
                                    ..default()
                                },

                                background_color: BackgroundColor(Color::hex("2D2D2D").unwrap()),
                                ..default()
                            },
                        ))
                        .with_children(|slots| {
                            for (slot_number, slot) in local_inventory.iter().enumerate().skip(n_slots_per_row) {
                                create_inventory_slot(inventory_holder, slot_number, slots, slot.as_ref(), text_style.clone());
                            }
                        });

                    parent
                        .spawn((
                            Name::new("Hotbar Slots"),
                            NodeBundle {
                                style: Style {
                                    display: Display::Flex,
                                    height: Val::Px(5.0 + slot_size),
                                    border: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(5.0), Val::Px(0.0)),

                                    ..default()
                                },
                                border_color: BorderColor(Color::hex("222222").unwrap()),
                                background_color: BackgroundColor(Color::WHITE),
                                ..default()
                            },
                            UiImage {
                                texture: asset_server.load("cosmos/images/ui/inventory-footer.png"),
                                ..Default::default()
                            },
                        ))
                        .with_children(|slots| {
                            for (slot_number, slot) in local_inventory.iter().enumerate().take(n_slots_per_row) {
                                create_inventory_slot(inventory_holder, slot_number, slots, slot.as_ref(), text_style.clone());
                            }
                        });
                });
        }
    }
}

#[derive(Debug, Component, Reflect)]
struct DisplayedItemFromInventory {
    inventory_holder: Entity,
    slot_number: usize,
    item_stack: Option<ItemStack>,
}

fn on_update_inventory(
    mut commands: Commands,
    query: Query<(Entity, &Inventory), Changed<Inventory>>,
    asset_server: Res<AssetServer>,
    mut current_slots: Query<(Entity, &mut DisplayedItemFromInventory)>,
) {
    for (entity, inventory) in query.iter() {
        for (display_entity, mut displayed_slot) in current_slots
            .iter_mut()
            .filter(|(_, x)| x.inventory_holder == entity && x.item_stack.as_ref() != inventory.itemstack_at(x.slot_number))
        {
            let font = asset_server.load("fonts/PixeloidSans.ttf");

            let text_style = TextStyle {
                color: Color::WHITE,
                font_size: 22.0,
                font: font.clone(),
            };

            displayed_slot.item_stack = inventory.itemstack_at(displayed_slot.slot_number).cloned();

            if let Some(item_stack) = displayed_slot.item_stack.as_ref() {
                let mut ecmds = commands.entity(display_entity);

                // removes previous rendered item here
                ecmds.despawn_descendants();

                create_item_stack_slot_data(item_stack, &mut ecmds, text_style);
            } else {
                commands.entity(display_entity).despawn_descendants();
            }
        }
    }
}

fn create_inventory_slot(
    inventory_holder: Entity,
    slot_number: usize,
    slots: &mut ChildBuilder,
    item_stack: Option<&ItemStack>,
    text_style: TextStyle,
) {
    let mut ecmds = slots.spawn((
        Name::new("Inventory Hotar Item"),
        NodeBundle {
            style: Style {
                // margin: UiRect::new(Val::Px(0.0), Val::Px(20.0), Val::Px(0.0), Val::Px(0.0)),
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                ..default()
            },

            border_color: BorderColor(Color::hex("222222").unwrap()),
            ..default()
        },
        DisplayedItemFromInventory {
            inventory_holder,
            slot_number,
            item_stack: item_stack.cloned(),
        },
    ));

    if let Some(item_stack) = item_stack {
        create_item_stack_slot_data(item_stack, &mut ecmds, text_style);
    }
}

fn create_item_stack_slot_data(item_stack: &ItemStack, ecmds: &mut EntityCommands, text_style: TextStyle) {
    ecmds.with_children(|p| {
        p.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(64.0),
                    height: Val::Px(64.0),
                    display: Display::Flex,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::FlexEnd,
                    ..Default::default()
                },
                ..Default::default()
            },
            RenderItem {
                item_id: item_stack.item_id(),
            },
        ))
        .with_children(|p| {
            p.spawn(TextBundle {
                style: Style {
                    margin: UiRect::new(Val::Px(0.0), Val::Px(5.0), Val::Px(0.0), Val::Px(5.0)),
                    ..default()
                },
                text: Text::from_section(format!("{} {}", item_stack.item_id(), item_stack.quantity()), text_style),
                ..default()
            });
        });
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            toggle_inventory,
            on_update_inventory,
            close_button_system,
            toggle_inventory_rendering,
        )
            .chain(),
    )
    .init_resource::<InventoryState>()
    .register_type::<DisplayedItemFromInventory>();
}
