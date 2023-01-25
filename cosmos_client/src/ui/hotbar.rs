use bevy::prelude::*;
use cosmos_core::{inventory::Inventory, item::Item};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    lang::Lang,
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

const ITEM_NAME_FADE_DURATION_SEC: f32 = 5.0;

#[derive(Component)]
pub struct Hotbar {
    // slot, slot text
    slots: Vec<(Entity, Entity)>,
    pub selected_slot: usize,
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

    #[inline]
    pub fn item_at_inventory_slot(&self, slot: usize, inv: &Inventory) -> usize {
        inv.len() - self.max_slots + slot
    }

    #[inline]
    pub fn item_at_selected_inventory_slot(&self, inv: &Inventory) -> usize {
        self.item_at_inventory_slot(self.selected_slot, inv)
    }
}

#[derive(Component)]
struct ItemNameDisplay;

fn image_path(selected: bool) -> &'static str {
    if selected {
        "images/ui/hotbar-slot-selected.png"
    } else {
        "images/ui/hotbar-slot.png"
    }
}

fn listen_button_presses(
    keys: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    input_handler: Res<CosmosInputHandler>,
    mut hotbar: Query<&mut Hotbar>,
) {
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot1, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 0;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot2, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 1;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot3, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 2;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot4, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 3;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot5, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 4;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot6, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 5;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot7, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 6;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot8, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 7;
        }
    }
    if input_handler.check_just_pressed(CosmosInputs::HotbarSlot9, &keys, &mouse) {
        if let Ok(mut hotbar) = hotbar.get_single_mut() {
            hotbar.selected_slot = 8;
        }
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
    query_inventory: Query<&Inventory, (Changed<Inventory>, With<LocalPlayer>)>,
    inventory_unchanged: Query<&Inventory, With<LocalPlayer>>,
    asset_server: Res<AssetServer>,
    mut text_query: Query<&mut Text>,
    item_name_query: Query<Entity, With<ItemNameDisplay>>,
    mut commands: Commands,
    names: Res<Lang<Item>>,
) {
    if let Ok(mut hb) = query_hb.get_single_mut() {
        if hb.selected_slot != hb.prev_slot {
            commands
                .entity(hb.slots[hb.prev_slot].0)
                .insert(UiImage(asset_server.load(image_path(false))));

            commands
                .entity(hb.slots[hb.selected_slot].0)
                .insert(UiImage(asset_server.load(image_path(true))));

            hb.prev_slot = hb.selected_slot;

            if let Ok(inv) = inventory_unchanged.get_single() {
                if let Ok(ent) = item_name_query.get_single() {
                    if let Ok(mut name_text) = text_query.get_mut(ent) {
                        if let Some(is) = inv.itemstack_at(hb.item_at_selected_inventory_slot(inv))
                        {
                            name_text.sections[0].value = names
                                .get_name_from_numeric_id(is.item_id())
                                .unwrap_or(&" ".to_owned())
                                .to_owned();

                            name_text.sections[0].style.color = Color::WHITE;
                        } else {
                            name_text.sections[0].value = "".to_owned();
                        }
                    }
                }
            }
        }

        if let Ok(inv) = query_inventory.get_single() {
            for hb_slot in 0..hb.max_slots {
                let is = inv.itemstack_at(hb.item_at_inventory_slot(hb_slot, inv));

                if let Ok(mut text) = text_query.get_mut(hb.slots[hb_slot].1) {
                    if let Some(is) = is {
                        text.sections[0].value = format!("{}", is.quantity());
                    } else {
                        text.sections[0].value = "".into();
                    }
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
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextBundle {
                    style: Style {
                        position: UiRect {
                            bottom: Val::Px(75.0),
                            ..default()
                        },

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
                    .with_alignment(TextAlignment::CENTER),
                    ..default()
                })
                .insert(ItemNameDisplay);
        });
}

fn add_hotbar(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            let mut hotbar = Hotbar::default();

            let mut slots = parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            });

            slots.with_children(|parent| {
                for slot_num in 0..hotbar.max_slots {
                    let path = image_path(hotbar.selected_slot == slot_num);

                    let mut slot = parent.spawn(ImageBundle {
                        image: asset_server.load(path).into(),
                        style: Style {
                            size: Size::new(Val::Px(64.0), Val::Px(64.0)),
                            ..default()
                        },
                        ..default()
                    });

                    let mut text_entity = None;

                    slot.with_children(|slot| {
                        // https://github.com/bevyengine/bevy/pull/5070
                        // In bevy 0.10 there will be a TextureAtlasLayout that can be used in GUIs to render the item's texture
                        // Until bevy 0.10, the hotbar will show no textures

                        text_entity = Some(
                            slot.spawn(TextBundle {
                                style: Style {
                                    position: UiRect {
                                        bottom: Val::Px(5.0),
                                        right: Val::Px(5.0),
                                        ..default()
                                    },
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
                                .with_alignment(TextAlignment::BOTTOM_RIGHT),
                                ..default()
                            })
                            .id(),
                        );
                    });

                    hotbar.slots.push((
                        slot.id(),
                        text_entity.expect("This should have been set in the closure above"),
                    ));
                }
            });

            slots.insert(hotbar);
        });
}

pub fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_enter(GameState::Playing)
            .with_system(add_hotbar)
            .with_system(add_item_text),
    )
    .add_system_set(
        SystemSet::on_update(GameState::Playing)
            .with_system(listen_for_change_events)
            .with_system(listen_button_presses)
            .with_system(tick_text_alpha_down),
    );
}
