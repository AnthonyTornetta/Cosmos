//! Displays the player's hotbar

use std::marker::PhantomData;

use bevy::{input::mouse::MouseWheel, prelude::*};
use cosmos_core::{
    block::block_events::BlockMessagesSet,
    inventory::{Inventory, held_item_slot::HeldItemSlot, itemstack::ItemStack},
    item::{Item, usable::cooldown::ItemCooldown},
    netty::client::LocalPlayer,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    structure::ship::ui::system_hotbar::SystemSelectionSet,
    ui::item_renderer::RenderItemCooldown,
};

use super::{
    components::show_cursor::no_open_menus,
    font::DefaultFont,
    item_renderer::{NoHoverTooltip, RenderItem},
};

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
    /// This does NOT delete the items from the world - since a hotbar is typically a visual
    /// representation of the items, not the items themselves
    pub fn clear_contents(&mut self) {
        self.items = vec![None; self.items.len()];
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

struct HotbarEntities {
    slot: Entity,
    slot_text: Entity,
    render_item_ent: Entity,
}

#[derive(Component)]
/// The hotbar the player can see
pub struct Hotbar {
    slots: Vec<HotbarEntities>,
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

    /// The slots a hotbar covers are from 0..max_slots
    pub fn n_slots(&self) -> usize {
        self.max_slots
    }

    /// Sets the slot selected by the user to this value
    pub fn set_selected_slot(&mut self, slot: usize) {
        debug_assert!(slot < self.max_slots, "Hotbar slot too big! {slot} must be < {}", self.max_slots);
        self.selected_slot = slot;
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
    mut scroll_evr: MessageReader<MouseWheel>,
    mut q_held_item_slot: Query<&mut HeldItemSlot, With<LocalPlayer>>,
    mut hotbar: Query<&mut Hotbar>,
) {
    let Ok(mut hotbar) = hotbar.single_mut() else {
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

    let Ok(mut held_item_slot) = q_held_item_slot.single_mut() else {
        return;
    };

    if hotbar.selected_slot as u32 != held_item_slot.slot() {
        held_item_slot.set_slot(hotbar.selected_slot as u32);
    }
}

fn tick_text_alpha_down(mut query: Query<&mut TextColor, With<ItemNameDisplay>>, time: Res<Time>) {
    if let Ok(mut text) = query.single_mut() {
        let col: Srgba = text.as_ref().0.into();

        text.as_mut().0 = Srgba {
            red: col.red,
            green: col.green,
            blue: col.blue,
            alpha: (col.alpha - time.delta_secs() / ITEM_NAME_FADE_DURATION_SEC).max(0.0),
        }
        .into();
    }
}

fn listen_for_change_events(
    mut query_hb: Query<&mut Hotbar>,
    query_inventory: Query<&HotbarContents, (Changed<HotbarContents>, With<LocalPlayerHotbar>)>,
    inventory_unchanged: Query<&HotbarContents, With<LocalPlayerHotbar>>,
    asset_server: Res<AssetServer>,
    mut text_query: Query<(&mut Text, &mut TextColor)>,
    item_name_query: Query<Entity, With<ItemNameDisplay>>,
    mut commands: Commands,
    names: Res<Lang<Item>>,
    items: Res<Registry<Item>>,
) {
    let Ok(mut hb) = query_hb.single_mut() else {
        return;
    };

    if hb.selected_slot != hb.prev_slot {
        commands
            .entity(hb.slots[hb.prev_slot].slot)
            .insert(ImageNode::new(asset_server.load(image_path(false))));

        commands
            .entity(hb.slots[hb.selected_slot].slot)
            .insert(ImageNode::new(asset_server.load(image_path(true))));

        hb.prev_slot = hb.selected_slot;

        if let Ok(inv) = inventory_unchanged.single()
            && let Ok(ent) = item_name_query.single()
            && let Ok((mut name_text, mut name_color)) = text_query.get_mut(ent)
        {
            if let Some(is) = inv.itemstack_at(hb.selected_slot()) {
                names
                    .get_name_from_numeric_id(is.item_id())
                    .unwrap_or(items.from_numeric_id(is.item_id()).unlocalized_name())
                    .clone_into(&mut name_text.as_mut().0);

                name_color.as_mut().0 = Color::WHITE;
            } else {
                "".clone_into(&mut name_text.as_mut().0);
            }
        }
    }

    if let Ok(hotbar_contents) = query_inventory.single() {
        for hb_slot in 0..hb.max_slots {
            let is = hotbar_contents.itemstack_at(hb_slot);

            if let Ok((mut text, _)) = text_query.get_mut(hb.slots[hb_slot].slot_text) {
                if let Some(is) = is {
                    if is.quantity() != 1 {
                        text.as_mut().0 = format!("{}", is.quantity());
                    } else {
                        text.as_mut().0 = "".into();
                    }
                } else {
                    text.as_mut().0 = "".into();
                }
            }
        }
    }
}

fn add_item_text(mut commands: Commands, default_font: Res<DefaultFont>) {
    let text_font = TextFont {
        font_size: 24.0,
        font: default_font.0.clone(),
        ..Default::default()
    };

    commands
        .spawn((
            Name::new("Item hotbar text"),
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        bottom: Val::Px(75.0),
                        position_type: PositionType::Absolute,

                        ..default()
                    },
                    Text::new(""),
                    text_font,
                    TextLayout {
                        justify: JustifyText::Center,
                        ..Default::default()
                    },
                ))
                .insert(ItemNameDisplay);
        });
}

fn populate_hotbar(
    q_hotbar_contents: Query<&HotbarContents, (Changed<HotbarContents>, With<LocalPlayerHotbar>)>,
    hotbar: Query<&Hotbar>,
    q_item_cooldown: Query<&ItemCooldown>,
    q_render_item: Query<&RenderItem>,
    mut commands: Commands,
) {
    let Ok(hotbar) = hotbar.single() else {
        warn!("Missing hotbar");
        return;
    };

    let Ok(hotbar_contents) = q_hotbar_contents.single() else {
        return;
    };

    for (item, hotbar_ents) in hotbar_contents.iter().take(hotbar.slots.len()).zip(hotbar.slots.iter()) {
        let Some(item_stack) = item else {
            commands.entity(hotbar_ents.render_item_ent).remove::<RenderItem>();

            continue;
        };

        if q_render_item
            .get(hotbar_ents.render_item_ent)
            .map(|x| x.item_id != item_stack.item_id())
            .unwrap_or(true)
        {
            let mut ecmds = commands.entity(hotbar_ents.render_item_ent);
            ecmds.insert((
                NoHoverTooltip,
                RenderItem {
                    item_id: item_stack.item_id(),
                },
            ));
            if let Some(cooldown) = item_stack.query_itemstack_data(&q_item_cooldown).copied() {
                ecmds.insert(RenderItemCooldown::new(cooldown));
            }
        }
    }
}

fn monitor_cooldown(
    q_item_cooldown: Query<&ItemCooldown>,
    mut q_render_cooldown: Query<&mut RenderItemCooldown>,
    q_contents: Query<(&Hotbar, &HotbarContents)>,
    mut commands: Commands,
) {
    for (hotbar, content) in q_contents.iter() {
        for (item, hotbar_ents) in content
            .iter()
            .take(hotbar.slots.len())
            .zip(hotbar.slots.iter())
            .flat_map(|(item, hb)| item.as_ref().map(|i| (i, hb)))
        {
            let Some(cooldown) = item.query_itemstack_data(&q_item_cooldown).copied() else {
                commands.entity(hotbar_ents.render_item_ent).remove::<RenderItemCooldown>();
                continue;
            };

            if let Ok(mut cd) = q_render_cooldown.get_mut(hotbar_ents.render_item_ent) {
                if cd.0 != cooldown {
                    cd.0 = cooldown;
                }
            } else {
                commands.entity(hotbar_ents.render_item_ent).insert(RenderItemCooldown(cooldown));
            }
        }
    }
}

fn add_hotbar(mut commands: Commands, default_font: Res<DefaultFont>, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Name::new("Hotbar Container"),
        ))
        .with_children(|parent| {
            let mut hotbar = Hotbar::default();

            let mut slots = parent.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                LocalPlayerHotbar,
                Name::new("Hotbar"),
            ));

            slots.with_children(|parent| {
                for slot_num in 0..hotbar.max_slots {
                    let path = image_path(hotbar.selected_slot == slot_num);

                    let mut slot = parent.spawn((
                        Name::new(format!("Slot {slot_num}")),
                        ImageNode::new(asset_server.load(path)),
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            ..default()
                        },
                    ));

                    let mut text_entity = None;
                    let mut item_entity = None;

                    slot.with_children(|slot| {
                        item_entity = Some(
                            slot.spawn((
                                Node {
                                    flex_grow: 1.0,
                                    ..Default::default()
                                },
                                Name::new("Hotbar Item Slot"),
                            ))
                            .with_children(|slot| {
                                text_entity = Some(
                                    slot.spawn((
                                        Name::new("Item Text"),
                                        Node {
                                            bottom: Val::Px(5.0),
                                            right: Val::Px(5.0),
                                            position_type: PositionType::Absolute,

                                            ..default()
                                        },
                                        Text::new(""),
                                        TextFont {
                                            font_size: 24.0,
                                            font: default_font.0.clone(),
                                            ..Default::default()
                                        },
                                        TextLayout {
                                            justify: JustifyText::Right,
                                            ..Default::default()
                                        },
                                    ))
                                    .id(),
                                );
                            })
                            .id(),
                        );
                    });

                    hotbar.slots.push(HotbarEntities {
                        slot: slot.id(),
                        slot_text: text_entity.expect("This should have been set in the closure above"),
                        render_item_ent: item_entity.expect("Should have been set above"),
                    });
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
    if let Ok((player_ent, hotbar)) = q_player.single() {
        commands
            .entity(player_ent)
            .insert((HotbarPriorityQueue::default(), HotbarContents::new(hotbar.max_slots)));
    }
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

fn sync_hotbar_to_inventory(
    q_inventory: Query<&Inventory, With<LocalPlayer>>,
    q_inventory_changed: Query<(), (Changed<Inventory>, With<LocalPlayer>)>,
    q_priority_changed: Query<(), (Changed<HotbarPriorityQueue>, With<LocalPlayerHotbar>)>,
    mut q_hotbar: Query<(&HotbarPriorityQueue, &mut HotbarContents), With<LocalPlayerHotbar>>,
) {
    let Ok((hotbar_prio_queue, mut hotbar_contents)) = q_hotbar.single_mut() else {
        return;
    };

    if hotbar_prio_queue.active() != Some(INVENTORY_PRIORITY_IDENTIFIER) {
        return;
    }

    if q_inventory_changed.is_empty() && q_priority_changed.is_empty() {
        return;
    }

    let Ok(inventory) = q_inventory.single() else {
        return;
    };

    let n_slots = hotbar_contents.n_slots();

    for (slot, itemstack) in inventory.iter().take(n_slots).enumerate() {
        hotbar_contents.set_itemstack_at(slot, itemstack.clone());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), (add_hotbar, add_item_text))
        .add_systems(
            Update,
            (
                add_inventory_to_priority_queue,
                add_hotbar_contents_to_player,
                sync_hotbar_to_inventory.after(BlockMessagesSet::SendMessagesForNextFrame),
                populate_hotbar,
                listen_for_change_events,
                monitor_cooldown,
                listen_button_presses.run_if(no_open_menus),
                tick_text_alpha_down,
            )
                .before(SystemSelectionSet::ApplyUserChanges)
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(is_hotbar_enabled),
        );
}
