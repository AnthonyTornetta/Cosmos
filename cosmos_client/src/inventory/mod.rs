//! Renders the inventory slots and handles all the logic for moving items around

use bevy::prelude::*;
use cosmos_core::{ecs::NeedsDespawned, inventory::Inventory};

use crate::{
    input::inputs::{CosmosInputHandler, CosmosInputs},
    netty::flags::LocalPlayer,
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

fn toggle_inventory_rendering(
    open_inventory: Query<Entity, With<RenderedInventory>>,
    inventory_state: Res<InventoryState>,
    mut commands: Commands,

    local_inventory: Query<&Inventory, With<LocalPlayer>>,
) {
    if !inventory_state.is_changed() {
        return;
    }

    match *inventory_state {
        InventoryState::Closed => {
            if let Ok(entity) = open_inventory.get_single() {
                commands.entity(entity).insert(NeedsDespawned);
            }
        }
        InventoryState::Open => {
            commands
                .spawn((
                    Name::new("Rendered Inventory"),
                    RenderedInventory,
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.4)),
                        ..default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Px(300.0),
                            height: Val::Px(10000.0),
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(1.0, 1.0, 1.0, 1.0)),
                        ..default()
                    },));
                });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (toggle_inventory, toggle_inventory_rendering).chain())
        .init_resource::<InventoryState>();
}
