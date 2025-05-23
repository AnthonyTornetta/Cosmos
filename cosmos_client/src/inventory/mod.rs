//! Renders the inventory slots and handles all the logic for moving items around

use bevy::{ecs::system::EntityCommands, prelude::*, window::PrimaryWindow};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::{
        block_events::BlockEventsSet,
        data::{BlockData, BlockDataIdentifier},
    },
    ecs::NeedsDespawned,
    inventory::{
        HeldItemStack, Inventory,
        held_item_slot::HeldItemSlot,
        itemstack::ItemStack,
        netty::{ClientInventoryMessages, InventoryIdentifier},
    },
    netty::{NettyChannelClient, client::LocalPlayer, cosmos_encoder, sync::mapping::NetworkMapping, system_sets::NetworkingSystemsSet},
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            scollable_container::ScrollBox,
            show_cursor::no_open_menus,
            window::{GuiWindow, UiWindowSystemSet},
        },
        item_renderer::{NoHoverTooltip, RenderItem},
    },
};

pub mod netty;

fn get_server_inventory_identifier(entity: Entity, mapping: &NetworkMapping, q_block_data: &Query<&BlockData>) -> InventoryIdentifier {
    if let Ok(block_data) = q_block_data.get(entity) {
        let structure_ent = mapping
            .server_from_client(&block_data.identifier.block.structure())
            .expect("Unable to map inventory to server inventory");

        let mut block = block_data.identifier.block;
        block.set_structure(structure_ent);
        InventoryIdentifier::BlockData(BlockDataIdentifier {
            block,
            block_id: block_data.identifier.block_id,
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
    open_inventories: Query<Entity, With<InventoryNeedsDisplayed>>,
    open_menus: Query<(), With<OpenMenu>>,
    inputs: InputChecker,
) {
    if inputs.check_just_pressed(CosmosInputs::ToggleInventory) {
        if !open_inventories.is_empty() {
            open_inventories.iter().for_each(|ent| {
                commands.entity(ent).remove::<InventoryNeedsDisplayed>();
            });
        } else if let Ok(player_inventory_ent) = player_inventory.get_single() {
            if open_menus.is_empty() {
                commands
                    .entity(player_inventory_ent)
                    .insert(InventoryNeedsDisplayed::Normal(InventorySide::Left));
            }
        }
    } else if inputs.check_just_pressed(CosmosInputs::Interact) && !open_inventories.is_empty() {
        open_inventories.iter().for_each(|ent| {
            commands.entity(ent).remove::<InventoryNeedsDisplayed>();
        });
    }
}

fn close_button_system(
    mut commands: Commands,
    q_close_inventory: Query<&RenderedInventory, With<NeedsDespawned>>,
    open_inventories: Query<Entity, With<InventoryNeedsDisplayed>>,
) {
    for rendered_inventory in q_close_inventory.iter() {
        if let Some(mut _ecmds) = commands.get_entity(rendered_inventory.inventory_holder) {
            open_inventories.iter().for_each(|ent| {
                commands.entity(ent).remove::<InventoryNeedsDisplayed>();
            });
        }
    }
}

#[derive(Debug, Clone)]
/// Instructions on how to render this inventory.
pub struct CustomInventoryRender {
    slots: Vec<(usize, Entity)>,
}

impl CustomInventoryRender {
    /// The slots should be a Vec<(slot_index, slot_entity)>.
    ///
    /// Each `slot_index` should be based off the slots in the inventory you wish to render.
    ///
    /// Each `slot_entity` should be a UI node that will be filled in to be an interactable item
    /// slot.
    pub fn new(slots: Vec<(usize, Entity)>) -> Self {
        Self { slots }
    }
}

#[derive(Component, Debug, Clone)]
/// Add this to an inventory you want displayed, and remove this component when you want to hide the inventory
pub enum InventoryNeedsDisplayed {
    /// A standard inventory rendering with no custom rendering. This Will be rendered like a chest
    /// or player's inventory.
    Normal(InventorySide),
    /// You dictate where and which inventory slots should be rendered. See
    /// [`CustomInventoryRender::new`]
    Custom(CustomInventoryRender),
}

impl Default for InventoryNeedsDisplayed {
    fn default() -> Self {
        Self::Normal(Default::default())
    }
}

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

#[derive(Component)]
struct InventoryRenderedItem;

fn toggle_inventory_rendering(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    added_inventories: Query<(Entity, &Inventory, &InventoryNeedsDisplayed, Option<&OpenInventoryEntity>), Added<InventoryNeedsDisplayed>>,
    mut without_needs_displayed_inventories: Query<
        (Entity, &mut Inventory, Option<&OpenInventoryEntity>),
        Without<InventoryNeedsDisplayed>,
    >,
    mut holding_item: Query<(Entity, &DisplayedItemFromInventory, &mut HeldItemStack), With<FollowCursor>>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    mut removed_components: RemovedComponents<InventoryNeedsDisplayed>,
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

        commands.entity(inventory_holder).remove::<OpenInventoryEntity>();
        if let Some(mut ecmds) = commands.get_entity(open_ent.0) {
            ecmds.insert(NeedsDespawned);
        }

        if let Ok((entity, displayed_item, mut held_item_stack)) = holding_item.get_single_mut() {
            let server_inventory_holder = get_server_inventory_identifier(inventory_holder, &mapping, &q_block_data);

            // Try to put it in its original spot first
            let leftover = local_inventory.insert_itemstack_at(displayed_item.slot_number, &held_item_stack, &mut commands);

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
                let (leftover, _) = local_inventory.insert_itemstack(&held_item_stack, &mut commands);

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
                    warn!(
                        "Unable to put itemstack into inventory it was taken out of - and dropping hasn't been implemented yet. Deleting for now."
                    );
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

        let text_style = TextFont {
            font_size: 22.0,
            font: font.clone(),
            ..Default::default()
        };

        let needs_displayed_side = match needs_displayed {
            InventoryNeedsDisplayed::Custom(slots) => {
                for &(slot_number, slot) in slots.slots.iter() {
                    commands.entity(slot).with_children(|p| {
                        let slot = inventory.itemstack_at(slot_number);

                        create_inventory_slot(inventory_holder, slot_number, p, slot, text_style.clone());
                    });
                }

                continue;
            }
            InventoryNeedsDisplayed::Normal(needs_displayed_side) => needs_displayed_side,
        };

        let inventory_border_size = 2.0;
        let n_slots_per_row: usize = 9;
        let slot_size = 64.0;
        let scrollbar_width = 15.0;

        let (left, right) = if *needs_displayed_side == InventorySide::Right {
            (Val::Auto, Val::Px(100.0))
        } else {
            (Val::Px(100.0), Val::Auto)
        };

        let width = Val::Px(n_slots_per_row as f32 * slot_size + inventory_border_size * 2.0 + scrollbar_width);

        let priority_slots = inventory.priority_slots();

        let border_color = BorderColor(Srgba::hex("222222").unwrap().into());

        const MAX_INVENTORY_HEIGHT_PX: f32 = 500.0;

        let non_hotbar_height = (((inventory.len() as f32 - inventory.priority_slots().map(|x| x.len()).unwrap_or(0) as f32) / 9.0).ceil()
            * INVENTORY_SLOTS_DIMS)
            .min(MAX_INVENTORY_HEIGHT_PX);

        let inv_ent = commands
            .spawn((
                Name::new("Rendered Inventory Title Bar"),
                RenderedInventory { inventory_holder },
                OpenMenu::new(0),
                BorderColor(Color::BLACK),
                GuiWindow {
                    title: inventory.name().into(),
                    body_styles: Node {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                },
                Node {
                    position_type: PositionType::Absolute,
                    right,
                    left,
                    top: Val::Px(100.0),
                    width,
                    border: UiRect::all(Val::Px(inventory_border_size)),
                    ..default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Rendered Inventory Non-Hotbar Slots"),
                    border_color,
                    BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                    ScrollBox::default(),
                    Node {
                        border: UiRect::horizontal(Val::Px(inventory_border_size)),
                        height: Val::Px(non_hotbar_height),
                        ..default()
                    },
                ))
                .with_children(|p| {
                    p.spawn(Node {
                        display: Display::Grid,
                        flex_grow: 1.0,
                        grid_column: GridPlacement::end(n_slots_per_row as i16),
                        grid_template_columns: vec![RepeatedGridTrack::px(GridTrackRepetition::Count(n_slots_per_row as u16), slot_size)],
                        ..Default::default()
                    })
                    .with_children(|slots| {
                        for (slot_number, slot) in inventory
                            .iter()
                            .enumerate()
                            .filter(|(slot, _)| priority_slots.as_ref().map(|x| !x.contains(slot)).unwrap_or(true))
                        {
                            create_inventory_slot(inventory_holder, slot_number, slots, slot.as_ref(), text_style.clone());
                        }
                    });
                });

                if let Some(priority_slots) = priority_slots {
                    p.spawn((
                        Name::new("Rendered Inventory Hotbar Slots"),
                        border_color,
                        Node {
                            display: Display::Flex,
                            border: UiRect::new(
                                Val::Px(inventory_border_size),
                                Val::Px(inventory_border_size),
                                Val::Px(5.0),
                                Val::Px(inventory_border_size),
                            ),
                            ..default()
                        },
                        ImageNode::new(asset_server.load("cosmos/images/ui/inventory-footer.png")),
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

        commands.entity(inventory_holder).insert(OpenInventoryEntity(inv_ent));
    }
}

fn drop_item(
    input_checker: InputChecker,
    q_inventory: Query<(Entity, &Inventory, &HeldItemSlot), With<LocalPlayer>>,
    mut client: ResMut<RenetClient>,
    network_mapping: Res<NetworkMapping>,
) {
    if !input_checker.check_just_pressed(CosmosInputs::DropItem) {
        return;
    }

    let Ok((local_player_entity, inventory, held_item_slot)) = q_inventory.get_single() else {
        return;
    };

    let selected_slot = held_item_slot.slot() as usize;
    let Some(is) = inventory.itemstack_at(selected_slot) else {
        return;
    };

    let Some(server_player_ent) = network_mapping.server_from_client(&local_player_entity) else {
        return;
    };

    client.send_message(
        NettyChannelClient::Inventory,
        cosmos_encoder::serialize(&ClientInventoryMessages::ThrowItemstack {
            quantity: if input_checker.check_pressed(CosmosInputs::BulkDropFlag) {
                is.quantity()
            } else {
                1
            },
            slot: selected_slot as u32,
            inventory_holder: InventoryIdentifier::Entity(server_player_ent),
        }),
    );
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

        let text_style = TextFont {
            font_size: 22.0,
            font: font.clone(),
            ..Default::default()
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

const INVENTORY_SLOTS_DIMS: f32 = 64.0;

fn create_inventory_slot(
    inventory_holder: Entity,
    slot_number: usize,
    slots: &mut ChildBuilder,
    item_stack: Option<&ItemStack>,
    text_style: TextFont,
) {
    let mut ecmds = slots.spawn((
        Name::new("Inventory Item"),
        InventoryItemMarker,
        Node {
            border: UiRect::all(Val::Px(2.0)),
            width: Val::Px(INVENTORY_SLOTS_DIMS),
            height: Val::Px(INVENTORY_SLOTS_DIMS),
            ..default()
        },
        BorderColor(Srgba::hex("222222").unwrap().into()),
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

    let text_style = TextFont {
        font_size: 22.0,
        font: font.clone(),
        ..Default::default()
    };

    let mut ecmds = commands.spawn((FollowCursor, NoHoverTooltip));

    create_item_stack_slot_data(&new_is, &mut ecmds, text_style, pickup_quantity);

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
    open_inventories: Query<Entity, With<InventoryNeedsDisplayed>>,
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
                inventory
                    .auto_move(slot_num, quantity, &mut commands)
                    .expect("Bad inventory slot values");
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
            if inventory.can_move_itemstack_to(&held_item_stack, clicked_slot) {
                let move_quantity = if lmb { held_item_stack.quantity() } else { 1 };

                let mut moving_itemstack = held_item_stack.clone();
                moving_itemstack.set_quantity(move_quantity);

                let over_quantity = held_item_stack.quantity() - move_quantity;

                let leftover = inventory.insert_itemstack_at(clicked_slot, &moving_itemstack, &mut commands);

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

                    let mut moving_itemstack = held_item_stack.clone();
                    moving_itemstack.set_quantity(quantity);

                    let unused_itemstack = held_item_stack.quantity() - quantity;

                    let leftover = inventory.insert_itemstack_at(clicked_slot, &moving_itemstack, &mut commands);

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
                    inventory.set_itemstack_at(clicked_slot, is_here, &mut commands);
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

fn create_item_stack_slot_data(item_stack: &ItemStack, ecmds: &mut EntityCommands, text_style: TextFont, quantity: u16) {
    ecmds
        .insert((
            Name::new("Render Item"),
            Node {
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                display: Display::Flex,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::FlexEnd,
                ..Default::default()
            },
            InventoryRenderedItem,
            RenderItem {
                item_id: item_stack.item_id(),
            },
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    margin: UiRect::new(Val::Px(0.0), Val::Px(5.0), Val::Px(0.0), Val::Px(5.0)),
                    ..default()
                },
                Text::new(format!("{quantity}")),
                text_style,
            ));
        });
}

fn follow_cursor(mut query: Query<&mut Node, With<FollowCursor>>, primary_window_query: Query<&Window, With<PrimaryWindow>>) {
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
    UpdateInventory,
    HandleInteractions,
    FollowCursor,
    ToggleInventoryRendering,
    MoveWindows,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        (
            (
                InventorySet::ToggleInventory,
                InventorySet::UpdateInventory,
                InventorySet::HandleInteractions,
                InventorySet::FollowCursor,
                InventorySet::ToggleInventoryRendering,
            )
                .before(UiSystemSet::PreDoUi)
                .after(BlockEventsSet::SendEventsForNextFrame)
                .chain(),
            InventorySet::MoveWindows
                .in_set(UiSystemSet::DoUi)
                .after(UiWindowSystemSet::SendWindowEvents),
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            drop_item.run_if(no_open_menus),
            (toggle_inventory, close_button_system)
                .chain()
                .in_set(InventorySet::ToggleInventory),
            on_update_inventory.in_set(InventorySet::UpdateInventory),
            handle_interactions.in_set(InventorySet::HandleInteractions),
            follow_cursor.in_set(InventorySet::FollowCursor),
            toggle_inventory_rendering.in_set(InventorySet::ToggleInventoryRendering),
        )
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<DisplayedItemFromInventory>();

    netty::register(app);
}
