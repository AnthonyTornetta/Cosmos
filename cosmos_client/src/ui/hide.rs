//! Player-toggled hiding of UI

use bevy::prelude::*;

use crate::input::inputs::{CosmosInputs, InputChecker, InputHandler};

use super::OpenMenu;

#[derive(Resource)]
/// If this resource exists, HUD UI should be hidden.
///
/// This does NOT include menus like inventory/pause.
pub struct HideUi;

#[derive(Component, Debug, Default)]
/// For UI
///
/// If multiple things can hide this UI element, you can opt to use this component to hide it if
/// any reason is true.
pub struct HiddenReasons(Vec<&'static str>);

#[derive(Component)]
/// Signals this UI cannot be toggled
pub struct DontHideOnToggleUi;
#[derive(Component)]
/// Put this on a UI element that has a parent, but should still be toggled via the hide/show UI
/// toggle
pub struct HidableUiElement;

impl HiddenReasons {
    /// Adds a reason to hide this UI (make sure it's unique)
    pub fn add_reason(&mut self, reason: &'static str) {
        if self.0.contains(&reason) {
            return;
        }

        self.0.push(reason);
    }

    /// Checks if this UI has any reason to be hidden
    pub fn is_hidden(&self) -> bool {
        !self.0.is_empty()
    }

    /// Removes a reason for this to be hidden
    pub fn remove_reason(&mut self, reason: &'static str) {
        if let Some((idx, _)) = self.0.iter().enumerate().find(|(_, r)| **r == reason) {
            self.0.remove(idx);
        }
    }
}

fn on_hide_ui(
    hidden: Option<Res<HideUi>>,
    mut commands: Commands,
    inputs: InputChecker,
    mut q_ui: Query<
        (&mut Visibility, Option<&mut HiddenReasons>),
        (
            With<Node>,
            Or<(Without<Parent>, With<HidableUiElement>)>,
            Without<DontHideOnToggleUi>,
            Without<OpenMenu>,
        ),
    >,
) {
    if !inputs.check_just_pressed(CosmosInputs::HideUi) {
        return;
    }

    let hide = if hidden.is_some() {
        commands.remove_resource::<HideUi>();
        false
    } else {
        commands.insert_resource(HideUi);
        true
    };

    for (mut vis, reasons) in q_ui.iter_mut() {
        if hide {
            *vis = Visibility::Hidden;
            if let Some(mut r) = reasons {
                r.add_reason("cosmos:hide_ui");
            }
        } else {
            *vis = Visibility::Inherited;
            if let Some(mut r) = reasons {
                r.remove_reason("cosmos:hide_ui");
            }
        }
    }
}

fn on_change_reasons(mut q_ui: Query<(&mut Visibility, &HiddenReasons), (With<Node>, Changed<HiddenReasons>)>) {
    for (mut vis, hidden) in q_ui.iter_mut() {
        if hidden.is_hidden() {
            *vis = Visibility::Hidden;
        } else {
            *vis = Visibility::Inherited;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (on_hide_ui, on_change_reasons).chain());
}
