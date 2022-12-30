use bevy::prelude::*;

use crate::state::game_state::GameState;

#[derive(Component)]
struct Hotbar {
    slots: Vec<Entity>,
    selected_slot: usize,
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
            slots: Vec::with_capacity(max_slots),
        }
    }
}

fn listen_for_change_events() {}

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
                    let path = if hotbar.selected_slot == slot_num {
                        "images/ui/hotbar-slot-selected.png"
                    } else {
                        "images/ui/hotbar-slot.png"
                    };

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

fn update_hotbar_contents() {}

pub fn register(app: &mut App) {
    app.add_system_set(SystemSet::on_enter(GameState::Playing).with_system(add_hotbar))
        .add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(update_hotbar_contents),
        );
}
