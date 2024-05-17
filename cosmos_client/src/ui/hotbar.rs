//! Displays the player's hotbar

use std::marker::PhantomData;

use bevy::{input::mouse::MouseWheel, prelude::*};
use cosmos_core::{
    inventory::{held_item_slot::HeldItemSlot, itemstack::ItemStack, Inventory},
    item::Item,
    netty::client::LocalPlayer,
    registry::{identifiable::Identifiable, Registry},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    state::game_state::GameState,
};

use super::{components::show_cursor::no_open_menus, item_renderer::RenderItem};

const ITEM_NAME_FADE_DURATION_SEC: f32 = 5.0;

#[derive(Debug, Component)]
/// The hotbar that will be rendered for the player
///
/// This is to identify the entity with the player's hotbar.
pub struct LocalPlayerHotbar;

#[derive(Component, Default, Debug)]
/// The contents that should be displayed on the hotbar
pub struct HotbarContents {
    items: Vec<Option<ItemStack>>,
}

impl HotbarContents {
    /// Creates a new hotbar contents that can hold up to the specified size.
    pub fn new(n_slots: usize) -> Self {
        Self {
            items: vec![None; n_slots],
        }
    }

    /// Gets the item stack at this slot if there is one. If the slot is out of bounds, this will also return None.
    pub fn itemstack_at(&self, slot: usize) -> Option<&ItemStack> {
        self.items.get(slot).and_then(|x| x.as_ref())
    }

    /// Sets the itemstack at this slot. If slot is out of bounds, the program will panic.
    pub fn set_itemstack_at(&mut self, slot: usize, itemstack: Option<ItemStack>) {
        self.items[slot] = itemstack;
    }

    /// Clears any items that are in the contents
    ///
    /// Unless you have safely transferred the data of the ItemStacks within this
    /// inventory over to other entities, or manually removed those data entities
    /// from the world, you should provide this with the Some value for the commands argument.
    pub fn clear_contents(&mut self, remove_data_entities: Option<&mut Commands>) {
        if let Some(cmds) = remove_data_entities {
            for is in self.items.iter_mut().flat_map(|x| x) {
                is.remove(cmds);
            }
        }

        let n_slots = self.items.len();
        self.items = vec![None; n_slots];
    }

    /// Iterates over every slot
    pub fn iter(&self) -> std::slice::Iter<Option<ItemStack>> {
        self.items.iter()
    }

    /// Reports the number of slots this can hold
    pub fn n_slots(&self) -> usize {
        self.items.len()
    }
}

/// The priority queue for a hotbar
pub type HotbarPriorityQueue = PriorityQueue<HotbarContents>;

#[derive(Component, Debug)]
/// Represents who should have ownership over this component. This is useful when multiple
/// sources want to control the same thing, but one should have priority over the other.
pub struct PriorityQueue<HotbarContents> {
    _phantom: PhantomData<HotbarContents>,
    queue: Vec<(String, i32)>,
}

impl<T> PriorityQueue<T> {
    /// Adds an id to the priority queue.
    ///
    /// If the queue priority is the highest out of any other waiting, then ownership will be given to that id in the [`Self::active`] method.
    pub fn add(&mut self, id: impl Into<String>, queue_priority: i32) {
        self.queue.push((id.into(), queue_priority));
        // highest -> smallest
        self.queue.sort_by_key(|x| -x.1);
    }

    /// Returns the current id that has the highest priority
    pub fn active(&self) -> Option<&str> {
        self.queue.first().map(|x| x.0.as_str())
    }

    /// Removes this id from the priority queue
    ///
    /// Returns if that id was ever in the queue
    pub fn remove(&mut self, id: &str) -> bool {
        let Some((idx, _)) = self.queue.iter().enumerate().find(|(_, x)| x.0 == id) else {
            return false;
        };

        self.queue.remove(idx);
        // Will already be sorted, no need to re-sort

        true
    }
}

impl<T> Default for PriorityQueue<T> {
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
            queue: vec![],
        }
    }
}

#[derive(Component)]
/// The hotbar the player can see
pub struct Hotbar {
    /// Vec<(slot, slot text)>
    slots: Vec<(Entity, Entity)>,
    selected_slot: usize,
    prev_slot: usize,
    max_slots: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        Self::new(9)
    }
}

impl Hotbar {
    fn new(max_slots: usize) -> Self {
        Self {
            max_slots,
            selected_slot: 0,
            prev_slot: 0,
            slots: Vec::with_capacity(max_slots),
        }
    }

    /// This is the slot the player has currently selected - corresponds to the proper inventory slots
    pub fn selected_slot(&self) -> usize {
        self.selected_slot
    }
}

#[derive(Component)]
struct ItemNameDisplay;

fn image_path(selected: bool) -> &'static str {
    if selected {
        "cosmos/images/ui/hotbar-slot-selected.png"
    } else {
        "cosmos/images/ui/hotbar-slot.png"
    }
}

fn listen_button_presses(
    input_handler: InputChecker,
    mut scroll_evr: EventReader<MouseWheel>,
    mut q_held_item_slot: Query<&mut HeldItemSlot, With<LocalPlayer>>,
    mut hotbar: Query<&mut Hotbar>,
) {
    let Ok(mut hotbar) = hotbar.get_single_mut() else {
        return;
    };

    for ev in scroll_evr.read() {
        if ev.y > 0.0 {
            if hotbar.selected_slot == 0 {
                hotbar.selected_slot = hotbar.max_slots - 1;
            } else {
                hotbar.selected_slot -= 1;
            }
        } else if ev.y < 0.0 {
            if hotbar.selected_slot == hotbar.max_slots - 1 {
                hotbar.selected_slot = 0;
            } else {
                hotbar.selected_slot += 1;
            }
        }
    }

    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot1) {
        hotbar.selected_slot = 0;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot2) {
        hotbar.selected_slot = 1;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot3) {
        hotbar.selected_slot = 2;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot4) {
        hotbar.selected_slot = 3;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot5) {
        hotbar.selected_slot = 4;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot6) {
        hotbar.selected_slot = 5;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot7) {
        hotbar.selected_slot = 6;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot8) {
        hotbar.selected_slot = 7;
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot9) {
        hotbar.selected_slot = 8;
    }

    let Ok(mut held_item_slot) = q_held_item_slot.get_single_mut() else {
        return;
    };

    if hotbar.selected_slot as u32 != held_item_slot.slot() {
        held_item_slot.set_slot(hotbar.selected_slot as u32);
    }
}

fn tick_text_alpha_down(mut query: Query<&mut Text, With<ItemNameDisplay>>, time: Res<Time>) {
    if let Ok(mut text) = query.get_single_mut() {
        let col = text.sections[0].style.color;

        text.sections[0].style.color = Color::rgba(
            col.r(),
            col.g(),
            col.b(),
            (col.a() - time.delta_seconds() / ITEM_NAME_FADE_DURATION_SEC).max(0.0),
        );
    }
}

fn listen_for_change_events(
    mut query_hb: Query<&mut Hotbar>,
    query_inventory: Query<&HotbarContents, (Changed<HotbarContents>, With<LocalPlayerHotbar>)>,
    inventory_unchanged: Query<&HotbarContents, With<LocalPlayerHotbar>>,
    asset_server: Res<AssetServer>,
    mut text_query: Query<&mut Text>,
    item_name_query: Query<Entity, With<ItemNameDisplay>>,
    mut commands: Commands,
    names: Res<Lang<Item>>,
    items: Res<Registry<Item>>,
) {
    let Ok(mut hb) = query_hb.get_single_mut() else {
        return;
    };

    if hb.selected_slot != hb.prev_slot {
        commands
            .entity(hb.slots[hb.prev_slot].0)
            .insert(UiImage::new(asset_server.load(image_path(false))));

        commands
            .entity(hb.slots[hb.selected_slot].0)
            .insert(UiImage::new(asset_server.load(image_path(true))));

        hb.prev_slot = hb.selected_slot;

        if let Ok(inv) = inventory_unchanged.get_single() {
            if let Ok(ent) = item_name_query.get_single() {
                if let Ok(mut name_text) = text_query.get_mut(ent) {
                    if let Some(is) = inv.itemstack_at(hb.selected_slot()) {
                        names
                            .get_name_from_numeric_id(is.item_id())
                            .unwrap_or(items.from_numeric_id(is.item_id()).unlocalized_name())
                            .clone_into(&mut name_text.sections[0].value);

                        name_text.sections[0].style.color = Color::WHITE;
                    } else {
                        "".clone_into(&mut name_text.sections[0].value);
                    }
                }
            }
        }
    }

    if let Ok(hotbar_contents) = query_inventory.get_single() {
        for hb_slot in 0..hb.max_slots {
            let is = hotbar_contents.itemstack_at(hb_slot);

            if let Ok(mut text) = text_query.get_mut(hb.slots[hb_slot].1) {
                if let Some(is) = is {
                    if is.quantity() != 1 {
                        text.sections[0].value = format!("{}", is.quantity());
                    } else {
                        text.sections[0].value = "".into();
                    }
                } else {
                    text.sections[0].value = "".into();
                }
            }
        }
    }
}

fn add_item_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: Style {
                        bottom: Val::Px(75.0),
                        position_type: PositionType::Absolute,

                        ..default()
                    },
                    text: Text::from_section(
                        "",
                        TextStyle {
                            color: Color::WHITE,
                            font_size: 24.0,
                            font: asset_server.load("fonts/PixeloidSans.ttf"),
                        },
                    )
                    .with_justify(JustifyText::Center),
                    ..default()
                })
                .insert(ItemNameDisplay);
        });
}

fn populate_hotbar(
    q_hotbar_contents: Query<&HotbarContents, (Changed<HotbarContents>, With<LocalPlayerHotbar>)>,
    hotbar: Query<&Hotbar>,

    render_item_query: Query<&RenderItem>,
    mut commands: Commands,
) {
    let Ok(hotbar) = hotbar.get_single() else {
        warn!("Missing hotbar");
        return;
    };

    let Ok(hotbar_contents) = q_hotbar_contents.get_single() else {
        return;
    };

    for (item, &(slot_entity, _)) in hotbar_contents.iter().take(hotbar.slots.len()).zip(hotbar.slots.iter()) {
        let Some(item_stack) = item else {
            commands.entity(slot_entity).remove::<RenderItem>();

            continue;
        };

        if render_item_query
            .get(slot_entity)
            .map(|x| x.item_id != item_stack.item_id())
            .unwrap_or(true)
        {
            commands.entity(slot_entity).insert((
                RenderItem {
                    item_id: item_stack.item_id(),
                },
                Name::new("Hotbar Item Slot"),
            ));
        }
    }
}

fn add_hotbar(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexEnd,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            },
            Name::new("Hotbar Container"),
        ))
        .with_children(|parent| {
            let mut hotbar = Hotbar::default();

            let mut slots = parent.spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_grow: 1.0,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                },
                LocalPlayerHotbar,
                Name::new("Hotbar"),
            ));

            slots.with_children(|parent| {
                for slot_num in 0..hotbar.max_slots {
                    let path = image_path(hotbar.selected_slot == slot_num);

                    let mut slot = parent.spawn(ImageBundle {
                        image: asset_server.load(path).into(),
                        style: Style {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..default()
                        },
                        ..default()
                    });

                    let mut text_entity = None;

                    slot.with_children(|slot| {
                        text_entity = Some(
                            slot.spawn(TextBundle {
                                style: Style {
                                    bottom: Val::Px(5.0),
                                    right: Val::Px(5.0),
                                    position_type: PositionType::Absolute,

                                    ..default()
                                },
                                text: Text::from_section(
                                    "",
                                    TextStyle {
                                        color: Color::WHITE,
                                        font_size: 24.0,
                                        font: asset_server.load("fonts/PixeloidSans.ttf"),
                                    },
                                )
                                .with_justify(JustifyText::Right),
                                ..default()
                            })
                            .id(),
                        );
                    });

                    hotbar
                        .slots
                        .push((slot.id(), text_entity.expect("This should have been set in the closure above")));
                }
            });

            slots.insert(hotbar);
        });
}

#[derive(Component)]
/// If this component is present on any entity, the hotbar will not be rendered.
pub struct HotbarDisabled;

fn is_hotbar_enabled(q_hotbar_disabled: Query<(), With<HotbarDisabled>>) -> bool {
    q_hotbar_disabled.is_empty()
}

fn add_hotbar_contents_to_player(
    mut commands: Commands,
    q_player: Query<(Entity, &Hotbar), (With<LocalPlayerHotbar>, Without<HotbarContents>)>,
) {
    if let Ok((player_ent, hotbar)) = q_player.get_single() {
        commands
            .entity(player_ent)
            .insert((HotbarPriorityQueue::default(), HotbarContents::new(hotbar.max_slots)));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), (add_hotbar, add_item_text))
        .add_systems(
            Update,
            (
                add_inventory_to_priority_queue,
                add_hotbar_contents_to_player,
                sync_inventory,
                populate_hotbar,
                listen_for_change_events,
                listen_button_presses.run_if(no_open_menus),
                tick_text_alpha_down,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(is_hotbar_enabled),
        );
}

// move to separate file
const INVENTORY_PRIORITY_IDENTIFIER: &str = "cosmos:inventory";

fn add_inventory_to_priority_queue(
    mut q_added_queue: Query<&mut HotbarPriorityQueue, (With<LocalPlayerHotbar>, Added<HotbarPriorityQueue>)>,
) {
    for mut priority_queue in &mut q_added_queue {
        priority_queue.add(INVENTORY_PRIORITY_IDENTIFIER, 0);
    }
}

fn sync_inventory(
    q_inventory: Query<&Inventory, With<LocalPlayer>>,
    q_inventory_changed: Query<(), (Changed<Inventory>, With<LocalPlayer>)>,
    q_priority_changed: Query<(), (Changed<HotbarPriorityQueue>, With<LocalPlayerHotbar>)>,
    mut q_hotbar: Query<(&HotbarPriorityQueue, &mut HotbarContents), With<LocalPlayerHotbar>>,
) {
    let Ok((hotbar_prio_queue, mut hotbar_contents)) = q_hotbar.get_single_mut() else {
        return;
    };

    if hotbar_prio_queue.active() != Some(INVENTORY_PRIORITY_IDENTIFIER) {
        return;
    }

    if q_inventory_changed.is_empty() && q_priority_changed.is_empty() {
        return;
    }

    let Ok(inventory) = q_inventory.get_single() else {
        return;
    };

    let n_slots = hotbar_contents.n_slots();

    for (slot, itemstack) in inventory.iter().take(n_slots).enumerate() {
        hotbar_contents.set_itemstack_at(slot, itemstack.clone());
    }
}
