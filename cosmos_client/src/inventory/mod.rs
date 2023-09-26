//! Renders the inventory slots and handles all the logic for moving items around

use bevy::{ecs::system::EntityCommands, prelude::*};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    ecs::NeedsDespawned,
    inventory::{itemstack::ItemStack, netty::ClientInventoryMessages, Inventory},
    netty::{cosmos_encoder, NettyChannelClient},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::{flags::LocalPlayer, mapping::NetworkMapping},
    state::game_state::GameState,
    ui::item_renderer::RenderItem,
    window::setup::CursorFlags,
};

pub mod netty;

#[derive(Debug, Resource, Clone, Copy, Default)]
enum InventoryState {
    #[default]
    Closed,
    Open,
}

#[derive(Component)]
struct RenderedInventory;

fn toggle_inventory(mut inventory_state: ResMut<InventoryState>, inputs: InputChecker) {
    if inputs.check_just_pressed(CosmosInputs::ToggleInventory) {
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
    holding_item: Query<Entity, With<FollowCursor>>,
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
                for entity in holding_item.iter() {
                    commands.entity(entity).insert(NeedsDespawned);
                }

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

#[derive(Debug, Component, Reflect, Clone)]
struct DisplayedItemFromInventory {
    inventory_holder: Entity,
    slot_number: usize,
    item_stack: Option<ItemStack>,
}

fn on_update_inventory(
    mut commands: Commands,
    query: Query<(Entity, &Inventory), Changed<Inventory>>,
    asset_server: Res<AssetServer>,
    following_cursor: Query<&FollowCursor>,
    mut current_slots: Query<
        (Entity, &mut DisplayedItemFromInventory, Option<&FollowCursor>),
        Or<(With<InventoryItemMarker>, With<FollowCursor>)>,
    >,
) {
    for (entity, inventory) in query.iter() {
        for (display_entity, mut displayed_slot, follow_cursor) in current_slots
            .iter_mut()
            .filter(|(_, di, _)| di.inventory_holder == entity && di.item_stack.as_ref() != inventory.itemstack_at(di.slot_number))
        {
            println!("{:?} | {:?}", display_entity, displayed_slot.as_ref());

            // This is rarely hit, so putting this load in here is best
            let font = asset_server.load("fonts/PixeloidSans.ttf");

            let text_style = TextStyle {
                color: Color::WHITE,
                font_size: 22.0,
                font: font.clone(),
            };

            displayed_slot.item_stack = inventory.itemstack_at(displayed_slot.slot_number).cloned();

            let Some(mut ecmds) = commands.get_entity(display_entity) else {
                continue;
            };

            if let Some(item_stack) = displayed_slot.item_stack.as_ref() {
                // removes previous rendered item here
                ecmds.despawn_descendants();

                // Only create an item render here if we're not holding the item with our cursor (moving it around)
                if follow_cursor.is_some() || !following_cursor.iter().any(|x| x.slot == displayed_slot.slot_number) {
                    create_item_stack_slot_data(item_stack, &mut ecmds, text_style);
                }
            } else {
                ecmds.despawn_descendants();
            }
        }
    }
}

#[derive(Component, Debug)]
struct InventoryItemMarker;

fn create_inventory_slot(
    inventory_holder: Entity,
    slot_number: usize,
    slots: &mut ChildBuilder,
    item_stack: Option<&ItemStack>,
    text_style: TextStyle,
) {
    let mut ecmds = slots.spawn((
        Name::new("Inventory Item"),
        InventoryItemMarker,
        NodeBundle {
            style: Style {
                border: UiRect::all(Val::Px(2.0)),
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                ..default()
            },

            border_color: BorderColor(Color::hex("222222").unwrap()),
            ..default()
        },
        Interaction::None,
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

/**
 * Moving items around
 */

#[derive(Debug, Component)]
/// If something is tagged with this, it is being held and moved around by the player.
///
/// Note that even if something is being moved, it is still always within the player's inventory
struct FollowCursor {
    slot: usize,
}

fn pickup_item_into_cursor(children: Option<&Children>, displayed_item_clicked: &DisplayedItemFromInventory, commands: &mut Commands) {
    if !displayed_item_clicked.item_stack.is_some() {
        return;
    }

    let Some(children) = children else {
        return;
    };

    let Some(&child) = children.first() else {
        return;
    };

    commands
        .entity(child)
        .remove_parent()
        .insert((
            FollowCursor {
                slot: displayed_item_clicked.slot_number,
            },
            displayed_item_clicked.clone(),
        ))
        .log_components();
}

fn send_swap(
    client: &mut RenetClient,
    display_item_held: &DisplayedItemFromInventory,
    displayed_item_clicked: &DisplayedItemFromInventory,
    mapping: &NetworkMapping,
) {
    client.send_message(
        NettyChannelClient::Inventory,
        cosmos_encoder::serialize(&ClientInventoryMessages::SwapSlots {
            slot_a: display_item_held.slot_number as u32,
            inventory_a: mapping
                .server_from_client(&display_item_held.inventory_holder)
                .expect("Missing server entity for inventory"),
            slot_b: displayed_item_clicked.slot_number as u32,
            inventory_b: mapping
                .server_from_client(&displayed_item_clicked.inventory_holder)
                .expect("Missing server entity for inventory"),
        }),
    );
}

fn send_move(
    client: &mut RenetClient,
    display_item_held: &DisplayedItemFromInventory,
    displayed_item_clicked: &DisplayedItemFromInventory,
    mapping: &NetworkMapping,
    quantity: u16,
) {
    client.send_message(
        NettyChannelClient::Inventory,
        cosmos_encoder::serialize(&ClientInventoryMessages::MoveItemstack {
            from_slot: display_item_held.slot_number as u32,
            quantity,
            from_inventory: mapping
                .server_from_client(&display_item_held.inventory_holder)
                .expect("Missing server entity for inventory"),
            to_inventory: mapping
                .server_from_client(&displayed_item_clicked.inventory_holder)
                .expect("Missing server entity for inventory"),
            to_slot: displayed_item_clicked.slot_number as u32,
        }),
    )
}

fn handle_interactions(
    mut commands: Commands,
    following_cursor: Query<(Entity, &DisplayedItemFromInventory), With<FollowCursor>>,
    interactions: Query<(Entity, Option<&Children>, &DisplayedItemFromInventory, &Interaction), Without<FollowCursor>>,
    input_handler: InputChecker,
    mut inventory_query: Query<&mut Inventory>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
) {
    let lmb = input_handler.mouse_inputs().just_pressed(MouseButton::Left);
    let rmb = input_handler.mouse_inputs().just_pressed(MouseButton::Right);

    // Only runs as soon as the mouse is pressed, not every frame
    if !lmb && !rmb {
        return;
    }

    let Some((clicked_entity, children, displayed_item_clicked, _)) = interactions
        .iter()
        // hovered or pressed should trigger this because pressed doesn't detected right click
        .find(|(_, _, _, interaction)| !matches!(interaction, Interaction::None))
    else {
        return;
    };

    let bulk_moving = input_handler.check_pressed(CosmosInputs::AutoMoveItem);

    if bulk_moving {
        println!("BULK");
        let slot_num = displayed_item_clicked.slot_number;
        let inventory_entity = displayed_item_clicked.inventory_holder;

        if let Ok(mut inventory) = inventory_query.get_mut(inventory_entity) {
            let quantity = if lmb {
                u16::MAX
            } else {
                inventory
                    .itemstack_at(slot_num)
                    .map(|x| (x.quantity() as f32 / 2.0).ceil() as u16)
                    .unwrap_or(0)
            };

            inventory.auto_move(slot_num, quantity).expect("Bad inventory slot values");

            let server_entity = mapping
                .server_from_client(&displayed_item_clicked.inventory_holder)
                .expect("Missing server entity for inventory");

            client.send_message(
                NettyChannelClient::Inventory,
                cosmos_encoder::serialize(&ClientInventoryMessages::AutoMove {
                    from_slot: slot_num as u32,
                    quantity,
                    from_inventory: server_entity,
                    to_inventory: server_entity,
                }),
            );
        }
    } else if let Ok((following_entity, display_item_held)) = following_cursor.get_single() {
        let (slot_a, slot_b) = (display_item_held.slot_number, displayed_item_clicked.slot_number);

        if display_item_held.inventory_holder == displayed_item_clicked.inventory_holder {
            if let Ok(mut inventory) = inventory_query.get_mut(display_item_held.inventory_holder) {
                println!("A");

                let right_click_move = rmb && inventory.can_move_itemstack_to(slot_a, &inventory, slot_b);

                if right_click_move {
                    println!("RMB");
                    let quantity = if lmb { u16::MAX } else { 1 };

                    let leftover = inventory
                        .self_move_itemstack(slot_a, slot_b, quantity)
                        .expect("Bad inventory slot values");

                    send_move(&mut client, display_item_held, displayed_item_clicked, &mapping, quantity);

                    if leftover == 0 {
                        commands.entity(following_entity).insert(NeedsDespawned);
                    }
                } else if lmb {
                    println!("LMB");

                    inventory.self_swap_slots(slot_a, slot_b).expect("Bad inventory slot values");

                    send_swap(&mut client, display_item_held, displayed_item_clicked, &mapping);
                    // Pick up the item in the same space we just held, because the item we just placed has been moved there.
                    pickup_item_into_cursor(children, display_item_held, &mut commands);

                    commands
                        .entity(following_entity)
                        .remove::<FollowCursor>()
                        .set_parent(clicked_entity);
                } else {
                    println!("None... somehow");
                }
            }
        } else {
            if let Ok([mut inventory_a, mut inventory_b]) =
                inventory_query.get_many_mut([display_item_held.inventory_holder, displayed_item_clicked.inventory_holder])
            {
                println!("B");

                let can_move = rmb && inventory_a.can_move_itemstack_to(slot_a, &inventory_b, slot_b);

                if can_move {
                    let quantity = if lmb { u16::MAX } else { 1 };

                    let leftover = inventory_a
                        .move_itemstack(slot_a, &mut inventory_b, slot_b, quantity)
                        .expect("Bad inventory slot values");

                    send_move(&mut client, display_item_held, displayed_item_clicked, &mapping, quantity);

                    if leftover == 0 {
                        commands.entity(following_entity).insert(NeedsDespawned);
                    }
                } else if lmb {
                    inventory_a
                        .swap_slots(slot_a, &mut inventory_b, slot_b)
                        .expect("Bad inventory slot values");

                    send_swap(&mut client, display_item_held, displayed_item_clicked, &mapping);
                    // Pick up the item in the same space we just held, because the item we just placed has been moved there.
                    pickup_item_into_cursor(children, display_item_held, &mut commands);

                    commands
                        .entity(following_entity)
                        .remove::<FollowCursor>()
                        .set_parent(clicked_entity);
                }
            }
        }
    } else {
        pickup_item_into_cursor(children, displayed_item_clicked, &mut commands);
    }
}

/**
 * End moving items around
 */

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
            handle_interactions,
            close_button_system,
            toggle_inventory_rendering,
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<InventoryState>()
    .register_type::<DisplayedItemFromInventory>();

    netty::register(app);
}
