//! Displays the crosshair the player sees in & out of a ship

use bevy::prelude::*;
use cosmos_core::{netty::system_sets::NetworkingSystemsSet, state::GameState, utils::smooth_clamp::SmoothClamp};

use crate::window::setup::CursorFlagsSet;

fn add_crosshair(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            Name::new("Crosshair"),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    ImageNode::new(asset_server.load("cosmos/images/ui/crosshair.png")),
                    Node {
                        width: Val::Px(8.0),
                        height: Val::Px(8.0),
                        left: Val::Px(0.0),
                        right: Val::Px(0.0),
                        top: Val::Px(0.0),
                        bottom: Val::Px(0.0),
                        ..default()
                    },
                ))
                .insert(Crosshair);
        });
}

#[derive(Component)]
/// This is used for the UI element for the crosshair as an identifier
pub struct Crosshair;

fn update_cursor_pos(pos: Res<CrosshairOffset>, mut query: Query<&mut Node, With<Crosshair>>) {
    if let Ok(mut crosshair) = query.get_single_mut() {
        crosshair.left = Val::Px(pos.x);
        // bottom doesn't seem to work, so -pos.y is used.
        crosshair.top = Val::Px(-pos.y);
    }
}

#[derive(Default, Debug, Resource, Clone, Copy, Reflect)]
/// This represents how far away from the center the crosshair is
pub struct CrosshairOffset {
    /// How far from the center x the crosshair is
    pub x: f32,
    /// How far from the center y the crosshair is
    pub y: f32,
}

impl SmoothClamp for CrosshairOffset {
    fn smooth_clamp(&self, min: &Self, max: &Self, lerp: f32) -> Self {
        debug_assert!(min.x < max.x);
        debug_assert!(min.y < max.y);

        let mut res = *self;

        if self.x < min.x {
            res.x += (min.x - self.x) * lerp;
        } else if self.x > max.x {
            res.x += (max.x - self.x) * lerp;
        }

        if self.y < min.y {
            res.y += (min.y - self.y) * lerp;
        } else if self.y > max.y {
            res.y += (max.y - self.y) * lerp;
        }

        res
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Changing the offset of the crosshair (such as when piloting a ship) should be done here
pub enum CrosshairOffsetSet {
    /// Changing the offset of the crosshair (such as when piloting a ship) should be done here
    ApplyCrosshairChanges,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        CrosshairOffsetSet::ApplyCrosshairChanges.after(CursorFlagsSet::ApplyCursorFlagsUpdates),
    );

    app.insert_resource(CrosshairOffset::default())
        .add_systems(OnEnter(GameState::Playing), add_crosshair)
        .add_systems(
            Update,
            update_cursor_pos
                .after(CrosshairOffsetSet::ApplyCrosshairChanges)
                .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
                .in_set(NetworkingSystemsSet::Between)
                .run_if(in_state(GameState::Playing)),
        );
}
