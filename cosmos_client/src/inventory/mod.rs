//! Renders the inventory slots and handles all the logic for moving items around

use bevy::{ecs::system::EntityCommands, prelude::*, window::PrimaryWindow};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::data::{BlockData, BlockDataIdentifier},
    ecs::NeedsDespawned,
    inventory::{
        itemstack::ItemStack,
        netty::{ClientInventoryMessages, InventoryIdentifier},
        HeldItemStack, Inventory,
    },
    item::Item,
    netty::{cosmos_encoder, NettyChannelClient},
    registry::Registry,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    netty::{flags::LocalPlayer, mapping::NetworkMapping},
    state::game_state::GameState,
    ui::{
        components::window::{GuiWindow, WindowBundle},
        item_renderer::RenderItem,
        UiSystemSet,
    },
};

pub mod netty;

fn get_server_inventory_identifier(entity: Entity, mapping: &NetworkMapping, q_block_data: &Query<&BlockData>) -> InventoryIdentifier {
    if let Ok(block_data) = q_block_data.get(entity) {
        InventoryIdentifier::BlockData(BlockDataIdentifier {
            block: block_data.identifier.block,
            structure_entity: mapping
                .server_from_client(&block_data.identifier.structure_entity)
                .expect("Unable to map inventory to server inventory"),
        })
    } else {
        InventoryIdentifier::Entity(
            mapping
                .server_from_client(&entity)
                .expect("Unable to map inventory to server inventory"),
        )
    }
}

#[derive(Component)]
struct RenderedInventory {
    inventory_holder: Entity,
}

fn toggle_inventory(
    mut commands: Commands,
    player_inventory: Query<Entity, With<LocalPlayer>>,
    open_inventories: Query<Entity, With<NeedsDisplayed>>,
    inputs: InputChecker,
) {
    if inputs.check_just_pressed(CosmosInputs::ToggleInventory) {
        if !open_inventories.is_empty() {
            open_inventories.iter().for_each(|ent| {
                commands.entity(ent).remove::<NeedsDisplayed>();
            });
        } else if let Ok(player_inventory_ent) = player_inventory.get_single() {
            commands.entity(player_inventory_ent).insert(NeedsDisplayed(InventorySide::Left));
        }
    } else if inputs.check_just_pressed(CosmosInputs::Interact) && !open_inventories.is_empty() {
        open_inventories.iter().for_each(|ent| {
            commands.entity(ent).remove::<NeedsDisplayed>();
        });
    }
}

fn close_button_system(
    mut commands: Commands,
    q_close_inventory: Query<&RenderedInventory, With<NeedsDespawned>>,
    open_inventories: Query<Entity, With<NeedsDisplayed>>,
) {
    for rendered_inventory in q_close_inventory.iter() {
        // TODO: fix inventory closing to only close the one open
        if let Some(mut _ecmds) = commands.get_entity(rendered_inventory.inventory_holder) {
            open_inventories.iter().for_each(|ent| {
                commands.entity(ent).remove::<NeedsDisplayed>().log_components();
            });
        }
    }
}

#[derive(Default, Component)]
/// Add this to an inventory you want displayed, and remove this component when you want to hide the inventory
pub struct NeedsDisplayed(InventorySide);

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
/// The side of the screen the inventory will be rendered
pub enum InventorySide {
    #[default]
    /// Right side
    Right,
    /// Left side - used for the player's inventory, so prefer right generally.
    Left,
}

#[derive(Component)]
/// Holds a reference to the opened inventory GUI
struct OpenInventoryEntity(Entity);

fn toggle_inventory_rendering(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    added_inventories: Query<(Entity, &Inventory, &NeedsDisplayed, Option<&OpenInventoryEntity>), Added<NeedsDisplayed>>,
    mut without_needs_displayed_inventories: Query<(Entity, &mut Inventory, Option<&OpenInventoryEntity>), Without<NeedsDisplayed>>,
    mut holding_item: Query<(Entity, &DisplayedItemFromInventory, &mut HeldItemStack), With<FollowCursor>>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    mut removed_components: RemovedComponents<NeedsDisplayed>,
    q_block_data: Query<&BlockData>,
) {
    for removed in removed_components.read() {
        let Ok((inventory_holder, mut local_inventory, open_inventory_entity)) = without_needs_displayed_inventories.get_mut(removed)
        else {
            continue;
        };

        let Some(open_ent) = open_inventory_entity else {
            continue;
        };

        let entity = open_ent.0;

        commands.entity(inventory_holder).remove::<OpenInventoryEntity>();
        if let Some(mut ecmds) = commands.get_entity(entity) {
            ecmds.insert(NeedsDespawned);
        }

        if let Ok((entity, displayed_item, mut held_item_stack)) = holding_item.get_single_mut() {
            let server_inventory_holder = get_server_inventory_identifier(inventory_holder, &mapping, &q_block_data);

            // Try to put it in its original spot first
            let leftover = local_inventory.insert_item_stack_at(displayed_item.slot_number, &held_item_stack);

            if leftover != held_item_stack.quantity() {
                // Only send information to server if there is a point to the move
                held_item_stack.set_quantity(leftover);

                client.send_message(
                    NettyChannelClient::Inventory,
                    cosmos_encoder::serialize(&ClientInventoryMessages::DepositHeldItemstack {
                        inventory_holder: server_inventory_holder,
                        slot: displayed_item.slot_number as u32,
                        quantity: u16::MAX,
                    }),
                );
            }

            if !held_item_stack.is_empty() {
                // Put it wherever it can fit if it couldn't go back to its original spot
                let leftover = local_inventory.insert_itemstack(&held_item_stack);

                if leftover != held_item_stack.quantity() {
                    // Only send information to server if there is a point to the insertion
                    client.send_message(
                        NettyChannelClient::Inventory,
                        cosmos_encoder::serialize(&ClientInventoryMessages::InsertHeldItem {
                            inventory_holder: server_inventory_holder,
                            quantity: u16::MAX,
                        }),
                    );
                }

                if leftover != 0 {
                    warn!("Unable to put itemstack into inventory it was taken out of - and dropping hasn't been implemented yet. Deleting for now.");
                    // Only send information to server if there is a point to the insertion
                    client.send_message(
                        NettyChannelClient::Inventory,
                        cosmos_encoder::serialize(&ClientInventoryMessages::ThrowHeldItemstack { quantity: u16::MAX }),
                    );
                }
            }

            commands.entity(entity).insert(NeedsDespawned);
        }
    }

    for (inventory_holder, inventory, needs_displayed, open_inventory_entity) in added_inventories.iter() {
        if open_inventory_entity.is_some() {
            continue;
        }

        let font = asset_server.load("fonts/PixeloidSans.ttf");

        let text_style = TextStyle {
            color: Color::WHITE,
            font_size: 22.0,
            font: font.clone(),
        };

        let inventory_border_size = 2.0;
        let n_slots_per_row: usize = 9;
        let slot_size = 64.0;

        let (left, right) = if needs_displayed.0 == InventorySide::Right {
            (Val::Auto, Val::Px(100.0))
        } else {
            (Val::Px(100.0), Val::Auto)
        };

        let open_inventory = commands
            .spawn((
                Name::new("Rendered Inventory"),
                RenderedInventory { inventory_holder },
                WindowBundle {
                    window: GuiWindow {
                        title: inventory.name().into(),
                        body_styles: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    node_bundle: NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right,
                            left,
                            top: Val::Px(100.0),
                            width: Val::Px(n_slots_per_row as f32 * slot_size + inventory_border_size * 2.0),
                            border: UiRect::all(Val::Px(inventory_border_size)),
                            ..default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        ..default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                let priority_slots = inventory.priority_slots();

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

                            background_color: BackgroundColor(Color::hex("2D2D2D0A").unwrap()),
                            ..default()
                        },
                    ))
                    .with_children(|slots| {
                        for (slot_number, slot) in inventory
                            .iter()
                            .enumerate()
                            .filter(|(slot, _)| priority_slots.as_ref().map(|x| !x.contains(slot)).unwrap_or(true))
                        {
                            create_inventory_slot(inventory_holder, slot_number, slots, slot.as_ref(), text_style.clone());
                        }
                    });

                if let Some(priority_slots) = priority_slots {
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
                            for slot_number in priority_slots {
                                create_inventory_slot(
                                    inventory_holder,
                                    slot_number,
                                    slots,
                                    inventory.itemstack_at(slot_number),
                                    text_style.clone(),
                                );
                            }
                        });
                }
            })
            .id();

        commands.entity(inventory_holder).insert(OpenInventoryEntity(open_inventory));
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
    q_inventory: Query<(Entity, &Inventory), Changed<Inventory>>,
    mut held_item_query: Query<(Entity, &HeldItemStack, &mut DisplayedItemFromInventory), Changed<HeldItemStack>>,
    mut current_slots: Query<(Entity, &mut DisplayedItemFromInventory), Without<HeldItemStack>>,
    asset_server: Res<AssetServer>,
) {
    for (inventory_entity, inventory) in q_inventory.iter() {
        for (display_entity, mut displayed_slot) in current_slots.iter_mut() {
            if displayed_slot.inventory_holder == inventory_entity
                && displayed_slot.item_stack.as_ref() != inventory.itemstack_at(displayed_slot.slot_number)
            {
                displayed_slot.item_stack = inventory.itemstack_at(displayed_slot.slot_number).cloned();

                let Some(mut ecmds) = commands.get_entity(display_entity) else {
                    continue;
                };

                rerender_inventory_slot(&mut ecmds, &displayed_slot, &asset_server, true);
            }
        }
    }

    assert!(held_item_query.iter().count() <= 1, "BAD HELD ITEMS!");

    if let Ok((entity, held_item_stack, mut displayed_item)) = held_item_query.get_single_mut() {
        displayed_item.item_stack = Some(held_item_stack.0.clone());

        if let Some(mut ecmds) = commands.get_entity(entity) {
            rerender_inventory_slot(&mut ecmds, &displayed_item, &asset_server, false);
        }
    }
}

fn rerender_inventory_slot(
    ecmds: &mut EntityCommands,
    displayed_item: &DisplayedItemFromInventory,
    asset_server: &AssetServer,
    as_child: bool,
) {
    ecmds.despawn_descendants();

    let Some(is) = displayed_item.item_stack.as_ref() else {
        return;
    };

    let quantity = is.quantity();

    if quantity != 0 {
        // This is rarely hit, so putting this load in here is best
        let font = asset_server.load("fonts/PixeloidSans.ttf");

        let text_style = TextStyle {
            color: Color::WHITE,
            font_size: 22.0,
            font: font.clone(),
        };

        if as_child {
            ecmds.with_children(|p| {
                create_item_stack_slot_data(is, &mut p.spawn_empty(), text_style, quantity);
            });
        } else {
            create_item_stack_slot_data(is, ecmds, text_style, quantity);
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
        ecmds.with_children(|p| {
            let mut ecmds = p.spawn_empty();

            create_item_stack_slot_data(item_stack, &mut ecmds, text_style, item_stack.quantity());
        });
    }
}

/**
 * Moving items around
 */

#[derive(Debug, Component)]
/// If something is tagged with this, it is being held and moved around by the player.
///
/// Note that even if something is being moved, it is still always within the player's inventory
struct FollowCursor;

fn pickup_item_into_cursor(
    displayed_item_clicked: &DisplayedItemFromInventory,
    commands: &mut Commands,
    quantity_multiplier: f32,
    inventory: &mut Inventory,
    asset_server: &AssetServer,
    client: &mut RenetClient,
    server_inventory_holder: InventoryIdentifier,
) {
    let Some(is) = displayed_item_clicked.item_stack.as_ref() else {
        return;
    };

    let pickup_quantity = (quantity_multiplier * is.quantity() as f32).ceil() as u16;

    let mut new_is = is.clone();
    new_is.set_quantity(pickup_quantity);

    let displayed_item = DisplayedItemFromInventory {
        inventory_holder: displayed_item_clicked.inventory_holder,
        item_stack: Some(new_is.clone()),
        slot_number: displayed_item_clicked.slot_number,
    };

    let font = asset_server.load("fonts/PixeloidSans.ttf");

    let text_style = TextStyle {
        color: Color::WHITE,
        font_size: 22.0,
        font: font.clone(),
    };

    let mut ecmds = commands.spawn(FollowCursor);

    create_item_stack_slot_data(
        displayed_item.item_stack.as_ref().expect("This was added above"),
        &mut ecmds,
        text_style,
        pickup_quantity,
    );

    ecmds.insert((displayed_item, HeldItemStack(new_is)));

    let slot_clicked = displayed_item_clicked.slot_number;
    if let Some(is) = inventory.mut_itemstack_at(slot_clicked) {
        let leftover_quantity = is.quantity() - (is.quantity() as f32 * quantity_multiplier).ceil() as u16;
        is.set_quantity(leftover_quantity);

        if is.is_empty() {
            inventory.remove_itemstack_at(slot_clicked);
        }
    }

    client.send_message(
        NettyChannelClient::Inventory,
        cosmos_encoder::serialize(&ClientInventoryMessages::PickupItemstack {
            inventory_holder: server_inventory_holder,
            slot: slot_clicked as u32,
            quantity: pickup_quantity,
        }),
    );
}

fn handle_interactions(
    mut commands: Commands,
    mut following_cursor: Query<(Entity, &mut HeldItemStack)>,
    interactions: Query<(&DisplayedItemFromInventory, &Interaction), Without<FollowCursor>>,
    input_handler: InputChecker,
    mut inventory_query: Query<&mut Inventory>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    q_block_data: Query<&BlockData>,
    asset_server: Res<AssetServer>,
    items: Res<Registry<Item>>,
    open_inventories: Query<Entity, With<NeedsDisplayed>>,
) {
    let lmb = input_handler.mouse_inputs().just_pressed(MouseButton::Left);
    let rmb = input_handler.mouse_inputs().just_pressed(MouseButton::Right);

    // Only runs as soon as the mouse is pressed, not every frame
    if !lmb && !rmb {
        return;
    }

    let Some((displayed_item_clicked, _)) = interactions
        .iter()
        // hovered or pressed should trigger this because pressed doesn't detected right click
        .find(|(_, interaction)| !matches!(interaction, Interaction::None))
    else {
        return;
    };

    let bulk_moving = input_handler.check_pressed(CosmosInputs::AutoMoveItem);

    let server_inventory_holder = get_server_inventory_identifier(displayed_item_clicked.inventory_holder, &mapping, &q_block_data);

    if bulk_moving {
        let slot_num = displayed_item_clicked.slot_number;
        let inventory_entity = displayed_item_clicked.inventory_holder;

        // try to find non-self inventory first, then default to self
        let other_inventory = open_inventories.iter().find(|&x| x != inventory_entity).unwrap_or(inventory_entity);

        let other_inventory = get_server_inventory_identifier(other_inventory, &mapping, &q_block_data);

        if let Ok(mut inventory) = inventory_query.get_mut(inventory_entity) {
            let quantity = if lmb {
                u16::MAX
            } else {
                inventory
                    .itemstack_at(slot_num)
                    .map(|x| (x.quantity() as f32 / 2.0).ceil() as u16)
                    .unwrap_or(0)
            };

            if other_inventory == server_inventory_holder {
                inventory.auto_move(slot_num, quantity).expect("Bad inventory slot values");
            }
            // logic is handled on server otherwise, don't feel like copying it here

            client.send_message(
                NettyChannelClient::Inventory,
                cosmos_encoder::serialize(&ClientInventoryMessages::AutoMove {
                    from_slot: slot_num as u32,
                    quantity,
                    from_inventory: server_inventory_holder,
                    to_inventory: other_inventory,
                }),
            );
        }
    } else if let Ok((following_entity, mut held_item_stack)) = following_cursor.get_single_mut() {
        let clicked_slot = displayed_item_clicked.slot_number;

        if let Ok(mut inventory) = inventory_query.get_mut(displayed_item_clicked.inventory_holder) {
            let item = items.from_numeric_id(held_item_stack.item_id());

            if inventory.can_move_itemstack_to(&held_item_stack, clicked_slot) {
                let move_quantity = if lmb { held_item_stack.quantity() } else { 1 };
                let over_quantity = held_item_stack.quantity() - move_quantity;

                let leftover = inventory.insert_item_at(clicked_slot, item, move_quantity);

                held_item_stack.set_quantity(over_quantity + leftover);

                if held_item_stack.is_empty() {
                    commands.entity(following_entity).insert(NeedsDespawned);
                }

                client.send_message(
                    NettyChannelClient::Inventory,
                    cosmos_encoder::serialize(&ClientInventoryMessages::DepositHeldItemstack {
                        inventory_holder: server_inventory_holder,
                        slot: clicked_slot as u32,
                        quantity: move_quantity,
                    }),
                )
            } else {
                let is_here = inventory.remove_itemstack_at(clicked_slot);

                if is_here.as_ref().map(|is| is.quantity() == 1).unwrap_or(true) || lmb {
                    let quantity = if lmb { held_item_stack.quantity() } else { 1 };
                    let unused_itemstack = held_item_stack.quantity() - quantity;

                    let leftover = inventory.insert_item_at(clicked_slot, item, quantity);

                    assert_eq!(
                        leftover, 0,
                        "Leftover wasn't 0 somehow? This could only mean something has an invalid stack size"
                    );

                    held_item_stack.set_quantity(unused_itemstack);

                    if unused_itemstack == 0 {
                        if let Some(is_here) = is_here {
                            held_item_stack.0 = is_here;
                        } else {
                            commands.entity(following_entity).insert(NeedsDespawned);
                        }
                    }

                    let message = if lmb {
                        // A swap assumes we're depositing everything, which will remove all items on the server-side.
                        cosmos_encoder::serialize(&ClientInventoryMessages::DepositAndSwapHeldItemstack {
                            inventory_holder: server_inventory_holder,
                            slot: clicked_slot as u32,
                        })
                    } else {
                        cosmos_encoder::serialize(&ClientInventoryMessages::DepositHeldItemstack {
                            inventory_holder: server_inventory_holder,
                            slot: clicked_slot as u32,
                            quantity: 1,
                        })
                    };

                    client.send_message(NettyChannelClient::Inventory, message);
                } else {
                    inventory.set_itemstack_at(clicked_slot, is_here);
                }
            }
        }
    } else if let Ok(mut inventory) = inventory_query.get_mut(displayed_item_clicked.inventory_holder) {
        let quantity_multiplier = if lmb { 1.0 } else { 0.5 };

        pickup_item_into_cursor(
            displayed_item_clicked,
            &mut commands,
            quantity_multiplier,
            &mut inventory,
            &asset_server,
            &mut client,
            server_inventory_holder,
        );
    }
}

/**
 * End moving items around
 */

fn create_item_stack_slot_data(item_stack: &ItemStack, ecmds: &mut EntityCommands, text_style: TextStyle, quantity: u16) {
    ecmds
        .insert((
            Name::new("Render Item"),
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
                text: Text::from_section(format!("{quantity}"), text_style),
                ..default()
            });
        });
}

fn follow_cursor(mut query: Query<&mut Style, With<FollowCursor>>, primary_window_query: Query<&Window, With<PrimaryWindow>>) {
    let Some(Some(cursor_pos)) = primary_window_query.get_single().ok().map(|x| x.cursor_position()) else {
        return; // cursor is outside of window or the window was closed
    };
    for mut style in query.iter_mut() {
        style.position_type = PositionType::Absolute;
        style.left = Val::Px(cursor_pos.x - 32.0);
        style.top = Val::Px(cursor_pos.y - 32.0);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum InventorySet {
    ToggleInventory,
    FlushToggleInventory,
    UpdateInventory,
    FlushUpdateInventory,
    HandleInteractions,
    FlushHandleInteractions,
    FollowCursor,
    FlushFollowCursor,
    ToggleInventoryRendering,
    FlushToggleInventoryRendering,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            InventorySet::ToggleInventory,
            InventorySet::FlushToggleInventory,
            InventorySet::UpdateInventory,
            InventorySet::FlushUpdateInventory,
            InventorySet::HandleInteractions,
            InventorySet::FlushHandleInteractions,
            InventorySet::FollowCursor,
            InventorySet::FlushFollowCursor,
            InventorySet::ToggleInventoryRendering,
            InventorySet::FlushToggleInventoryRendering,
        )
            .before(UiSystemSet::ApplyDeferredA)
            .chain(),
    )
    .add_systems(
        Update,
        (
            // apply_deferred
            apply_deferred.in_set(InventorySet::FlushToggleInventory),
            apply_deferred.in_set(InventorySet::FlushUpdateInventory),
            apply_deferred.in_set(InventorySet::FlushHandleInteractions),
            apply_deferred.in_set(InventorySet::FlushFollowCursor),
            apply_deferred.in_set(InventorySet::FlushToggleInventoryRendering),
            // Logic
            (toggle_inventory, close_button_system).in_set(InventorySet::ToggleInventory),
            on_update_inventory.in_set(InventorySet::UpdateInventory),
            handle_interactions.in_set(InventorySet::HandleInteractions),
            follow_cursor.in_set(InventorySet::FollowCursor),
            toggle_inventory_rendering.in_set(InventorySet::ToggleInventoryRendering),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<DisplayedItemFromInventory>();

    netty::register(app);
}
