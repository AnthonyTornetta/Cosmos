use bevy::prelude::*;
use cosmos_core::inventory::Inventory;

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
    state::game_state::GameState,
};

#[derive(Component)]
struct Hotbar {
    slots: Vec<Entity>,
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
}

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

fn listen_for_change_events(
    mut query_hb: Query<&mut Hotbar>,
    query_inventory: Query<&Inventory, (Changed<Inventory>, With<LocalPlayer>)>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if let Ok(mut hb) = query_hb.get_single_mut() {
        if hb.selected_slot != hb.prev_slot {
            commands
                .entity(hb.slots[hb.prev_slot])
                .remove::<UiImage>()
                .insert(UiImage(asset_server.load(image_path(false)).into()));

            commands
                .entity(hb.slots[hb.selected_slot])
                .remove::<UiImage>()
                .insert(UiImage(asset_server.load(image_path(true)).into()));

            hb.prev_slot = hb.selected_slot;
        }
    }
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

                    hotbar.slots.push(
                        parent
                            .spawn(ImageBundle {
                                image: asset_server.load(path).into(),
                                style: Style {
                                    size: Size::new(Val::Px(64.0), Val::Px(64.0)),
                                    ..default()
                                },
                                ..default()
                            })
                            .id(),
                    );
                }
            });

            slots.insert(hotbar);
        });
}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(add_hotbar))
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(listen_for_change_events)
                .with_system(listen_button_presses),
        );
}
