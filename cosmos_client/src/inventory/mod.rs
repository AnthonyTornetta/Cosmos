//! Renders the inventory slots and handles all the logic for moving items around
//!
//! Sphagetti town

use bevy::{a11y::Focus, color::palettes::css, ecs::system::EntityCommands, prelude::*, window::PrimaryWindow};
use bevy_renet::renet::RenetClient;
use cosmos_core::{
    block::{
        block_events::BlockEventsSet,
        data::{BlockData, BlockDataIdentifier},
    },
    creative::{CreativeTrashHeldItem, GrabCreativeItemEvent},
    ecs::NeedsDespawned,
    entities::player::creative::Creative,
    inventory::{
        HeldItemStack, Inventory,
        held_item_slot::HeldItemSlot,
        itemstack::ItemStack,
        netty::{ClientInventoryMessages, InventoryIdentifier},
    },
    item::{Item, item_category::ItemCategory},
    netty::{
        NettyChannelClient,
        client::LocalPlayer,
        cosmos_encoder,
        sync::{events::client_event::NettyEventWriter, mapping::NetworkMapping},
        system_sets::NetworkingSystemsSet,
    },
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{
        OpenMenu, UiSystemSet,
        components::{
            button::{ButtonEvent, CosmosButton, register_button},
            scollable_container::ScrollBox,
            show_cursor::no_open_menus,
            text_input::{InputType, InputValue, TextInput},
            window::{GuiWindow, GuiWindowTitleBar, UiWindowSystemSet},
        },
        font::DefaultFont,
        item_renderer::{CustomHoverTooltip, NoHoverTooltip, RenderItem},
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
    focused: Res<Focus>,
    q_input: Query<(), With<TextInput>>,
) {
    // Don't toggle the inventory while typing in the search bar (or any other text box)
    if focused.map(|x| q_input.contains(x)).unwrap_or(false) {
        return;
    }

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

// fn create_creative_ui(mut commands: Commands, categories: Res<Registry<ItemCategory>>, items: Res<Registry<Item>>) {
//     let mut all_items: HashMap<u16, Vec<&Item>> = HashMap::default();
//     for item in items.iter() {
//         let Some(cat) = item.category() else {
//             continue;
//         };
//
//         let Some(category) = categories.from_id(cat) else {
//             error!("Invalid item category - {cat}");
//             continue;
//         };
//         all_items.entry(category.id()).or_default().push(item);
//     }
// }

#[derive(Debug, Event)]
struct ItemCategoryClickedEvent(Entity);

#[derive(Debug, Component, PartialEq, Eq)]
enum ItemCategoryMarker {
    Category(u16),
    Inventory,
    Search,
}

impl ButtonEvent for ItemCategoryClickedEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

#[derive(Component)]
struct SelectedTab(ItemCategoryMarker);

#[derive(Component)]
struct VisibileHeight(Val);

#[derive(Component)]
struct InventorySearchBar;

#[derive(Component)]
struct SearchGrid;

fn toggle_inventory_rendering(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    added_inventories: Query<
        (
            Entity,
            &Inventory,
            &InventoryNeedsDisplayed,
            Option<&OpenInventoryEntity>,
            Has<Creative>,
        ),
        Added<InventoryNeedsDisplayed>,
    >,
    without_needs_displayed_inventories: Query<(Entity, Option<&OpenInventoryEntity>), Without<InventoryNeedsDisplayed>>,
    q_children: Query<&Children>,
    q_displayed_item: Query<Entity, With<FollowCursor>>,
    mut client: ResMut<RenetClient>,
    mut removed_components: RemovedComponents<InventoryNeedsDisplayed>,
    categories: Res<Registry<ItemCategory>>,
    category_names: Res<Lang<ItemCategory>>,
    items: Res<Registry<Item>>,
    q_held_item: Query<&Inventory, With<HeldItemStack>>,
) {
    for removed in removed_components.read() {
        let Ok((inventory_holder, open_inventory_entity)) = without_needs_displayed_inventories.get(removed) else {
            continue;
        };

        let Some(open_ent) = open_inventory_entity else {
            continue;
        };

        commands.entity(inventory_holder).remove::<OpenInventoryEntity>();
        if let Some(mut ecmds) = commands.get_entity(open_ent.0) {
            ecmds.insert(NeedsDespawned);
        }

        if HeldItemStack::get_held_is_inventory(inventory_holder, &q_children, &q_held_item).is_some() {
            client.send_message(
                NettyChannelClient::Inventory,
                cosmos_encoder::serialize(&ClientInventoryMessages::DropOrDepositHeldItemstack),
            );
        }
        if let Ok(entity) = q_displayed_item.get_single() {
            commands.entity(entity).insert(NeedsDespawned);
        }
    }

    for (inventory_holder, inventory, needs_displayed, open_inventory_entity, is_creative) in added_inventories.iter() {
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

        let width = n_slots_per_row as f32 * slot_size + inventory_border_size * 2.0 + scrollbar_width;

        let priority_slots = inventory.priority_slots();

        let border_color = BorderColor(Srgba::hex("222222").unwrap().into());

        let non_hotbar_height = (((inventory.len() as f32 - inventory.priority_slots().map(|x| x.len()).unwrap_or(0) as f32) / 9.0).ceil()
            * INVENTORY_SLOTS_DIMS)
            .min(MAX_INVENTORY_HEIGHT_PX);

        let inv_ent = commands
            .spawn((
                Name::new("Rendered Inventory"),
                RenderedInventory { inventory_holder },
                OpenMenu::new(0),
                BorderColor(Color::BLACK),
                GuiWindow {
                    title: inventory.name().into(),
                    body_styles: Node {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    window_background: BackgroundColor(border_color.0.into()),
                },
                Node {
                    position_type: PositionType::Absolute,
                    right,
                    left,
                    top: Val::Px(100.0),
                    width: Val::Px(width),
                    border: UiRect::all(Val::Px(inventory_border_size)),
                    ..default()
                },
            ))
            .with_children(|p| {
                if is_creative {
                    let mut sorted_categories = categories.iter().collect::<Vec<_>>();
                    sorted_categories.sort_by_key(|x| x.unlocalized_name());

                    p.spawn((
                        Name::new("Search Bar"),
                        GuiWindowTitleBar,
                        InventorySearchBar,
                        Visibility::Hidden,
                        BackgroundColor(border_color.0.into()),
                        BorderColor(css::GREY.into()),
                        Node {
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(5.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            margin: UiRect::horizontal(Val::Px(10.0)),
                            ..Default::default()
                        },
                        TextInput {
                            input_type: InputType::Text { max_length: Some(20) },
                            ..Default::default()
                        },
                        TextFont {
                            font: font.clone_weak(),
                            font_size: 16.0,
                            ..Default::default()
                        },
                    ));

                    p.spawn((
                        Name::new("Creative Tabs Right"),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(width - inventory_border_size * 2.0),
                            width: Val::Px(64.0),
                            flex_direction: FlexDirection::Column,
                            border: UiRect::new(Val::Px(0.0), Val::Px(2.0), Val::Px(2.0), Val::Px(2.0)),
                            ..Default::default()
                        },
                        BorderColor(css::BLACK.into()),
                    ))
                    .with_children(|p| {
                        let storage_id = items.from_id("cosmos:shop").map(|x| x.id()).unwrap_or_default();

                        p.spawn((
                            RenderItem { item_id: storage_id },
                            CustomHoverTooltip::new("Search"),
                            Name::new("Search Button"),
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                ..Default::default()
                            },
                            ItemCategoryMarker::Search,
                            CosmosButton::<ItemCategoryClickedEvent> { ..Default::default() },
                            BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
                        ));
                    });

                    p.spawn((
                        Name::new("Creative Tabs"),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(-64.0),
                            width: Val::Px(64.0),
                            flex_direction: FlexDirection::Column,
                            border: UiRect::new(Val::Px(2.0), Val::Px(0.0), Val::Px(2.0), Val::Px(2.0)),
                            ..Default::default()
                        },
                        BorderColor(css::BLACK.into()),
                    ))
                    .with_children(|p| {
                        let storage_id = items.from_id("cosmos:storage").map(|x| x.id()).unwrap_or_default();

                        p.spawn((
                            RenderItem { item_id: storage_id },
                            CustomHoverTooltip::new("Inventory"),
                            Name::new("Main Inventory Button"),
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                // margin: UiRect::bottom(Val::Px(5.0)),
                                ..Default::default()
                            },
                            ItemCategoryMarker::Inventory,
                            CosmosButton::<ItemCategoryClickedEvent> { ..Default::default() },
                            BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
                        ));

                        for cat in sorted_categories.iter() {
                            let category_symbol = items.from_id(cat.item_icon_id()).map(|x| x.id()).unwrap_or_default();
                            let name = category_names.get_name_or_unlocalized(cat);

                            p.spawn((
                                RenderItem { item_id: category_symbol },
                                Name::new(format!("Category {name} button")),
                                CustomHoverTooltip::new(name),
                                Node {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    ..Default::default()
                                },
                                ItemCategoryMarker::Category(cat.id()),
                                CosmosButton::<ItemCategoryClickedEvent> { ..Default::default() },
                                BackgroundColor(Srgba::hex("2D2D2D").unwrap().into()),
                            ));
                        }
                    });

                    p.spawn((
                        Name::new("Rendered Item Category (Search)"),
                        border_color,
                        BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                        ScrollBox::default(),
                        VisibileHeight(Val::Px(non_hotbar_height)),
                        Node {
                            border: UiRect::horizontal(Val::Px(inventory_border_size)),
                            height: Val::Px(0.0),
                            ..default()
                        },
                        SelectedTab(ItemCategoryMarker::Search),
                        Visibility::Hidden,
                    ))
                    .with_children(|p| {
                        p.spawn((
                            SearchGrid,
                            Node {
                                display: Display::Grid,
                                grid_column: GridPlacement::end(n_slots_per_row as i16),
                                grid_template_columns: vec![RepeatedGridTrack::px(
                                    GridTrackRepetition::Count(n_slots_per_row as u16),
                                    slot_size,
                                )],
                                ..Default::default()
                            },
                        ))
                        .with_children(|slots| {
                            let mut sorted_items = items.iter().collect::<Vec<_>>();
                            sorted_items.sort_by_key(|x| x.unlocalized_name());
                            for item in sorted_items {
                                create_creative_slot(slots, item, text_style.clone());
                            }
                        });
                    });

                    for cat in sorted_categories.iter() {
                        p.spawn((
                            Name::new(format!("Rendered Item Category ({})", cat.unlocalized_name())),
                            border_color,
                            BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                            ScrollBox::default(),
                            VisibileHeight(Val::Px(non_hotbar_height)),
                            Node {
                                border: UiRect::horizontal(Val::Px(inventory_border_size)),
                                height: Val::Px(0.0),
                                ..default()
                            },
                            SelectedTab(ItemCategoryMarker::Category(cat.id())),
                            Visibility::Hidden,
                        ))
                        .with_children(|p| {
                            p.spawn(Node {
                                display: Display::Grid,
                                grid_column: GridPlacement::end(n_slots_per_row as i16),
                                grid_template_columns: vec![RepeatedGridTrack::px(
                                    GridTrackRepetition::Count(n_slots_per_row as u16),
                                    slot_size,
                                )],
                                ..Default::default()
                            })
                            .with_children(|slots| {
                                let mut sorted_items = items
                                    .iter()
                                    .filter(|x| x.category() == Some(cat.unlocalized_name()))
                                    .collect::<Vec<_>>();
                                sorted_items.sort_by_key(|x| x.unlocalized_name());
                                for item in sorted_items {
                                    create_creative_slot(slots, item, text_style.clone());
                                }
                            });
                        });
                    }
                }

                p.spawn((
                    Name::new("Rendered Inventory Non-Hotbar Slots"),
                    border_color,
                    BackgroundColor(Srgba::hex("3D3D3D").unwrap().into()),
                    ScrollBox::default(),
                    VisibileHeight(Val::Px(non_hotbar_height)),
                    Node {
                        border: UiRect::horizontal(Val::Px(inventory_border_size)),
                        height: Val::Px(non_hotbar_height),
                        ..default()
                    },
                    SelectedTab(ItemCategoryMarker::Inventory),
                    OpenTab,
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
                            // More compensating for the border
                            margin: UiRect::left(Val::Px(inventory_border_size)),
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
    mut current_slots: Query<(Entity, &mut DisplayedItemFromInventory)>,
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

const INVENTORY_SLOTS_DIMS: f32 = 64.0;
const MAX_INVENTORY_HEIGHT_PX: f32 = 500.0;

#[derive(Debug, Component, Reflect, Clone)]
struct CreativeItem {
    item_id: u16,
}

#[derive(Event, Debug)]
struct CreativeItemClickedEvent(Entity);

impl ButtonEvent for CreativeItemClickedEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

fn create_creative_slot(slots: &mut ChildBuilder, item: &Item, text_style: TextFont) {
    let mut ecmds = slots.spawn((
        Name::new("Creative Inventory Item"),
        Node {
            border: UiRect::all(Val::Px(2.0)),
            width: Val::Px(INVENTORY_SLOTS_DIMS),
            height: Val::Px(INVENTORY_SLOTS_DIMS),
            ..default()
        },
        BorderColor(Srgba::hex("222222").unwrap().into()),
        Interaction::None,
        CosmosButton::<CreativeItemClickedEvent>::default(),
        CreativeItem { item_id: item.id() },
    ));

    ecmds.with_children(|p| {
        let mut ecmds = p.spawn_empty();
        create_item_slot_data(item, &mut ecmds, text_style, 1);
    });
}

#[derive(Component)]
struct OpenTab;

fn on_click_creative_category(
    mut evr_click_creative_tab: EventReader<ItemCategoryClickedEvent>,
    q_item_category_marker: Query<&ItemCategoryMarker>,
    mut q_unopen_tab: Query<
        (Entity, &mut Node, &mut Visibility, &VisibileHeight, &SelectedTab),
        (Without<InventorySearchBar>, Without<OpenTab>),
    >,
    mut q_open_tab: Query<(Entity, &mut Node, &mut Visibility, &SelectedTab), (Without<InventorySearchBar>, With<OpenTab>)>,
    mut q_creative_search: Query<&mut Visibility, With<InventorySearchBar>>,
    mut commands: Commands,
) {
    for ev in evr_click_creative_tab.read() {
        let Ok(item_category) = q_item_category_marker.get(ev.0) else {
            continue;
        };
        if let Ok((entity, mut node, mut vis, i_category)) = q_open_tab.get_single_mut() {
            if i_category.0 == *item_category {
                continue;
            }

            if let Ok(mut c_search) = q_creative_search.get_single_mut() {
                *c_search = Visibility::Hidden;
            }
            node.height = Val::Px(0.0);
            *vis = Visibility::Hidden;
            commands.entity(entity).remove::<OpenTab>();
        }

        let Some((entity, mut node, mut vis, vis_height, _)) = q_unopen_tab.iter_mut().find(|x| x.4.0 == *item_category) else {
            error!("Bad state");
            continue;
        };

        if item_category == &ItemCategoryMarker::Search {
            if let Ok(mut c_search) = q_creative_search.get_single_mut() {
                *c_search = Visibility::Inherited;
            }
        }

        node.height = vis_height.0;
        *vis = Visibility::default();
        commands.entity(entity).insert(OpenTab);
    }
}

fn create_inventory_slot(
    inventory_holder: Entity,
    slot_number: usize,
    slots: &mut ChildBuilder,
    item_stack: Option<&ItemStack>,
    text_style: TextFont,
) {
    let mut ecmds = slots.spawn((
        Name::new("Inventory Item"),
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
    quantity_multiplier: f32,
    client: &mut RenetClient,
    server_inventory_holder: InventoryIdentifier,
) {
    let Some(is) = displayed_item_clicked.item_stack.as_ref() else {
        return;
    };

    let pickup_quantity = (quantity_multiplier * is.quantity() as f32).ceil() as u16;
    let slot_clicked = displayed_item_clicked.slot_number;

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
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
    q_children: Query<&Children, With<LocalPlayer>>,
    interactions: Query<(&DisplayedItemFromInventory, &Interaction), Without<FollowCursor>>,
    input_handler: InputChecker,
    mut inventory_query: Query<&mut Inventory, Without<HeldItemStack>>,
    mut client: ResMut<RenetClient>,
    mapping: Res<NetworkMapping>,
    q_block_data: Query<&BlockData>,
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

    let player_kids = q_children.get_single().expect("Player missing all children");
    let held_item_inv =
        HeldItemStack::get_held_is_inventory_from_children_mut(player_kids, &mut q_held_item).expect("Missing held item inventory");

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
    } else if let Some(held_item_stack) = held_item_inv.itemstack_at(0) {
        let clicked_slot = displayed_item_clicked.slot_number;

        if let Ok(mut inventory) = inventory_query.get_mut(displayed_item_clicked.inventory_holder) {
            if inventory.can_move_itemstack_to(held_item_stack, clicked_slot) {
                let move_quantity = if lmb { held_item_stack.quantity() } else { 1 };

                client.send_message(
                    NettyChannelClient::Inventory,
                    cosmos_encoder::serialize(&ClientInventoryMessages::DepositHeldItemstack {
                        inventory_holder: server_inventory_holder,
                        slot: clicked_slot as u32,
                        quantity: move_quantity,
                    }),
                );

                // let mut moving_itemstack = held_item_stack.clone();
                // moving_itemstack.set_quantity(move_quantity);
                //
                // let over_quantity = held_item_stack.quantity() - move_quantity;
                //
                // let leftover = inventory.insert_itemstack_at(clicked_slot, &moving_itemstack, &mut commands);
                //
                // held_item_stack.set_quantity(over_quantity + leftover);
                //
                // if !held_item_stack.is_empty() {
                //     held_item_inv.set_itemstack_at(0, Some(held_item_stack), &mut commands);
                // }
            } else {
                let is_here = inventory.remove_itemstack_at(clicked_slot);

                let message = if lmb || is_here.as_ref().map(|is| is.quantity() == 1).unwrap_or(true) {
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
                // let quantity = if lmb { held_item_stack.quantity() } else { 1 };
                //
                // let mut moving_itemstack = held_item_stack.clone();
                // moving_itemstack.set_quantity(quantity);
                //
                // let unused_itemstack = held_item_stack.quantity() - quantity;
                //
                // let leftover = inventory.insert_itemstack_at(clicked_slot, &moving_itemstack, &mut commands);
                //
                // assert_eq!(
                //     leftover, 0,
                //     "Leftover wasn't 0 somehow? This could only mean something has an invalid stack size"
                // );
                //
                // held_item_stack.set_quantity(unused_itemstack);
                //
                // if unused_itemstack == 0 {
                //     if let Some(is_here) = is_here {
                //         held_item_stack.0 = is_here;
                //     } else {
                //         commands.entity(following_entity).insert(NeedsDespawned);
                //     }
                // }
                //
                // let message = if lmb {
                //                         } else {
                //                         };

                client.send_message(NettyChannelClient::Inventory, message);
            }
            //else {
            //  inventory.set_itemstack_at(clicked_slot, is_here, &mut commands);
            //}
        }
    } else if inventory_query.contains(displayed_item_clicked.inventory_holder) {
        let quantity_multiplier = if lmb { 1.0 } else { 0.5 };

        pickup_item_into_cursor(displayed_item_clicked, quantity_multiplier, &mut client, server_inventory_holder);
    }
}

fn create_item_stack_slot_data(item: &ItemStack, ecmds: &mut EntityCommands, text_style: TextFont, quantity: u16) {
    create_item_slot_data_raw(item.item_id(), ecmds, text_style, quantity);
}
fn create_item_slot_data(item: &Item, ecmds: &mut EntityCommands, text_style: TextFont, quantity: u16) {
    create_item_slot_data_raw(item.id(), ecmds, text_style, quantity);
}

fn create_item_slot_data_raw(item_id: u16, ecmds: &mut EntityCommands, text_style: TextFont, quantity: u16) {
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
            RenderItem { item_id },
        ))
        .with_children(|p| {
            if quantity != 1 {
                p.spawn((
                    Node {
                        margin: UiRect::new(Val::Px(0.0), Val::Px(5.0), Val::Px(0.0), Val::Px(5.0)),
                        ..default()
                    },
                    Text::new(format!("{quantity}")),
                    text_style,
                ));
            }
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

fn on_click_creative_item(
    q_creative_item: Query<&CreativeItem>,
    mut evr_clicked_creative_item: EventReader<CreativeItemClickedEvent>,
    inputs: InputChecker,
    items: Res<Registry<Item>>,
    mut nevw_set_item: NettyEventWriter<GrabCreativeItemEvent>,
    mut nevw_trash_item: NettyEventWriter<CreativeTrashHeldItem>,
    q_children: Query<&Children, With<LocalPlayer>>,
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
) {
    for ev in evr_clicked_creative_item.read() {
        let Ok(item_id) = q_creative_item.get(ev.0).map(|x| x.item_id) else {
            error!("Bad item - {ev:?}");
            continue;
        };

        let mut quantity = if inputs.check_pressed(CosmosInputs::AutoMoveItem) {
            items.from_numeric_id(item_id).max_stack_size()
        } else {
            1
        };

        let Ok(lp_children) = q_children.get_single() else {
            return;
        };

        if let Some(inv) = HeldItemStack::get_held_is_inventory_from_children_mut(lp_children, &mut q_held_item) {
            if let Some(held_is) = inv.itemstack_at(0) {
                if held_is.item_id() != item_id {
                    nevw_trash_item.send_default();
                    continue;
                }
                quantity += held_is.quantity();
            }
        }

        nevw_set_item.send(GrabCreativeItemEvent { quantity, item_id });
    }
}

fn draw_held_item(
    q_changed_held_item: Query<&Parent, (Changed<Inventory>, With<HeldItemStack>)>,
    q_opened_inventories: Query<(), With<RenderedInventory>>,
    q_local_player: Query<(Entity, &Children), With<LocalPlayer>>,
    q_held_item: Query<&Inventory, With<HeldItemStack>>,
    mut commands: Commands,
    q_follow_cursor: Query<Entity, With<FollowCursor>>,
    default_font: Res<DefaultFont>,
) {
    if q_opened_inventories.is_empty() {
        if let Ok(ent) = q_follow_cursor.get_single() {
            commands.entity(ent).insert(NeedsDespawned);
        }
        return;
    }

    let Ok((local_ent, children)) = q_local_player.get_single() else {
        return;
    };

    if !q_changed_held_item.iter().any(|p| p.get() == local_ent) && !q_follow_cursor.is_empty() {
        return;
    }

    let Some(held_inv) = HeldItemStack::get_held_is_inventory_from_children(children, &q_held_item) else {
        return;
    };

    let Some(is) = held_inv.itemstack_at(0) else {
        if let Ok(ent) = q_follow_cursor.get_single() {
            commands.entity(ent).insert(NeedsDespawned);
        }
        return;
    };

    let mut ecmds = if let Ok(ent) = q_follow_cursor.get_single() {
        let mut ecmds = commands.entity(ent);
        ecmds.despawn_descendants();
        ecmds
    } else {
        commands.spawn((
            Node {
                width: Val::Px(64.0),
                height: Val::Px(64.0),
                position_type: PositionType::Absolute,
                ..Default::default()
            },
            FollowCursor,
            NoHoverTooltip,
            Name::new("Held Item Render"),
        ))
    };

    let text_style = TextFont {
        font_size: 22.0,
        font: default_font.0.clone_weak(),
        ..Default::default()
    };

    create_item_stack_slot_data(is, &mut ecmds, text_style, is.quantity());

    // if let Ok((ent, mut render_item)) = q_follow_cursor.get_single_mut() {
    //     if render_item.item_id != is.item_id() {
    //         render_item.item_id = is.item_id();
    //     }
    //     if render_item.quantity != is.quantity() {
    //         render_item.quantity = is.quantity();
    //     }
    // } else {
    // }
}

fn on_change_search(
    q_changed_search: Query<&InputValue, (With<InventorySearchBar>, Changed<InputValue>)>,
    q_rendered_category: Query<Entity, With<SearchGrid>>,
    items: Res<Registry<Item>>,
    font: Res<DefaultFont>,
    lang: Res<Lang<Item>>,
    mut commands: Commands,
) {
    let Ok(input_value) = q_changed_search.get_single() else {
        return;
    };

    let text_style = TextFont {
        font_size: 22.0,
        font: font.clone(),
        ..Default::default()
    };

    let Ok(search) = q_rendered_category.get_single() else {
        return;
    };

    commands.entity(search).despawn_descendants().with_children(|p| {
        let lower = input_value.value().to_lowercase();
        let mut sorted_items = items
            .iter()
            .filter(|x| lang.get_name_or_unlocalized(*x).to_lowercase().contains(&lower))
            .collect::<Vec<_>>();
        sorted_items.sort_by_key(|x| x.unlocalized_name());
        for item in sorted_items {
            create_creative_slot(p, item, text_style.clone());
        }
    });
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
            (
                toggle_inventory,
                close_button_system,
                on_click_creative_category,
                on_click_creative_item,
                draw_held_item,
                on_change_search,
            )
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

    register_button::<ItemCategoryClickedEvent>(app);
    register_button::<CreativeItemClickedEvent>(app);

    netty::register(app);
}
