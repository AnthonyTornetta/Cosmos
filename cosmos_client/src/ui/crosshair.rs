//! Displays the crosshair the player sees in & out of a ship

use bevy::prelude::*;
use cosmos_core::{state::GameState, utils::smooth_clamp::SmoothClamp};

use crate::{asset::asset_loader::load_assets, window::setup::CursorFlagsSet};

#[derive(PartialEq, Eq, Default, Reflect)]
enum State {
    #[default]
    Normal,
    Indicating,
}

#[derive(Default, Component, Reflect)]
/// Controls the type of crosshair being displayed
pub struct CrosshairState {
    indicating_requests: Vec<&'static str>,
    prev_state: State,
}

impl CrosshairState {
    /// Shows an indicator crosshair so long as this reason is valid
    pub fn request_indicating(&mut self, id: &'static str) {
        if !self.indicating_requests.contains(&id) {
            self.indicating_requests.push(id);
        }
    }

    /// Removes this reason from using an indicator crosshair. Once there are no reasons, the indicating crosshair will not be used.
    pub fn remove_indicating(&mut self, id: &'static str) {
        let Some((idx, _)) = self.indicating_requests.iter().enumerate().find(|(_, x)| **x == id) else {
            return;
        };
        self.indicating_requests.swap_remove(idx);
    }

    fn state(&self) -> State {
        if self.indicating_requests.is_empty() {
            State::Normal
        } else {
            State::Indicating
        }
    }

    fn is_different_state(&self) -> bool {
        self.prev_state != self.state()
    }

    fn update_prev_state(&mut self) {
        self.prev_state = self.state();
    }
}

fn add_crosshair(mut commands: Commands, crosshair_assets: Res<CrosshairAssets>) {
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
            parent.spawn((
                ImageNode::new(crosshair_assets.normal.clone_weak()),
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                    ..default()
                },
                CrosshairState::default(),
                Crosshair,
            ));
        });
}

fn on_change_crosshair_state(
    crosshair_assets: Res<CrosshairAssets>,
    mut q_changed_state: Query<(&mut CrosshairState, &mut Node, &mut ImageNode)>,
) {
    for (mut state, mut node, mut img) in q_changed_state.iter_mut() {
        if !state.is_different_state() {
            continue;
        }
        state.update_prev_state();
        img.image = match state.state() {
            State::Normal => {
                node.width = Val::Px(8.0);
                node.height = Val::Px(8.0);
                crosshair_assets.normal.clone_weak()
            }
            State::Indicating => {
                node.width = Val::Px(16.0);
                node.height = Val::Px(16.0);
                crosshair_assets.indicating.clone_weak()
            }
        };
    }
}

#[derive(Component)]
/// This is used for the UI element for the crosshair as an identifier
pub struct Crosshair;

fn update_cursor_pos(pos: Res<CrosshairOffset>, mut query: Query<&mut Node, With<Crosshair>>) {
    if let Ok(mut crosshair) = query.single_mut() {
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

#[derive(Resource)]
struct CrosshairAssets {
    normal: Handle<Image>,
    indicating: Handle<Image>,
}

pub(super) fn register(app: &mut App) {
    load_assets::<Image, CrosshairAssets, 2>(
        app,
        GameState::Loading,
        ["cosmos/images/ui/crosshair.png", "cosmos/images/ui/crosshair-indicating.png"],
        |mut commands, [(normal, _), (indicating, _)]| {
            commands.insert_resource(CrosshairAssets { normal, indicating });
        },
    );

    app.configure_sets(
        Update,
        CrosshairOffsetSet::ApplyCrosshairChanges.after(CursorFlagsSet::ApplyCursorFlagsUpdates),
    );

    app.insert_resource(CrosshairOffset::default())
        .add_systems(OnEnter(GameState::Playing), add_crosshair)
        .add_systems(
            Update,
            (update_cursor_pos, on_change_crosshair_state)
                .chain()
                .after(CrosshairOffsetSet::ApplyCrosshairChanges)
                .after(CursorFlagsSet::ApplyCursorFlagsUpdates)
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<CrosshairState>();
}
