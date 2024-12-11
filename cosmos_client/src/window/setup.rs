//! Responsible for the initial creation of the window

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow, WindowFocused},
};
use cosmos_core::state::GameState;
// use iyes_perf_ui::entries::PerfUiBundle;

#[derive(Resource, Copy, Clone)]
/// Resource containing the various flags about the cursor, like if it's hidden or not
pub struct CursorFlags {
    locked: bool,
    visible: bool,
}

impl CursorFlags {
    /// Toggles the cursor between being hidden + locked and shown + unlocked
    pub fn toggle(&mut self) {
        self.locked = !self.locked;
        self.visible = !self.visible;
    }

    /// Shows + unlocks the cursor
    pub fn show(&mut self) {
        self.locked = false;
        self.visible = true;
    }

    /// Hides + locks the cursor
    pub fn hide(&mut self) {
        self.locked = true;
        self.visible = false;
    }

    /// Returns true if the cursor is locked
    pub fn is_cursor_locked(&self) -> bool {
        self.locked
    }

    /// Returns true if the cursor is shown
    pub fn is_cursor_shown(&self) -> bool {
        self.visible
    }
}

#[derive(Resource, Debug, Clone, Copy, Default)]
/// How much the cursor has moved since the last frame
pub struct DeltaCursorPosition {
    /// Delta cursor x
    pub x: f32,
    /// Delta cursor y
    pub y: f32,
}

fn setup_window(mut primary_query: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = primary_query.get_single_mut().expect("Missing primary window.");

    window.title = "Cosmos".into();
    window.cursor_options.visible = false;
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
}

fn update_mouse_deltas(mut delta: ResMut<DeltaCursorPosition>, mut ev_mouse_motion: EventReader<MouseMotion>) {
    delta.x = 0.0;
    delta.y = 0.0;

    for ev in ev_mouse_motion.read() {
        delta.x += ev.delta.x;
        delta.y += -ev.delta.y;
    }
}

fn window_focus_changed(
    mut primary_query: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut ev_focus: EventReader<WindowFocused>,
    cursor_flags: Res<CursorFlags>,
) {
    let Ok((window_entity, mut window)) = primary_query.get_single_mut() else {
        return;
    };

    if let Some(ev) = ev_focus.read().find(|e| e.window == window_entity) {
        if ev.focused {
            apply_cursor_flags(&mut window, *cursor_flags);
        } else {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

fn apply_cursor_flags(window: &mut Window, cursor_flags: CursorFlags) {
    window.cursor_options.grab_mode = if cursor_flags.locked {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };
    window.cursor_options.visible = cursor_flags.visible;
}

fn apply_cursor_flags_on_change(cursor_flags: Res<CursorFlags>, mut primary_query: Query<&mut Window, With<PrimaryWindow>>) {
    let Ok(mut window) = primary_query.get_single_mut() else {
        return;
    };

    apply_cursor_flags(&mut window, *cursor_flags);
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Handles when the cursor should be shown/hidden
pub enum CursorFlagsSet {
    /// Additions/removals of the [`CursorUnlocker`] component should be done in or before this.
    UpdateCursorFlags,
    /// The cursor is shown/hidden based on the existence of any [`CursorUnlocker`] entities.
    ApplyCursorFlagsUpdates,
}

fn create_debug(_commands: Commands) {
    // commands.spawn(PerfUiBundle::default());
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CursorFlags {
        locked: true,
        visible: false,
    })
    .configure_sets(
        Update,
        (CursorFlagsSet::UpdateCursorFlags, CursorFlagsSet::ApplyCursorFlagsUpdates).chain(),
    )
    .insert_resource(DeltaCursorPosition { x: 0.0, y: 0.0 })
    .add_systems(Startup, setup_window)
    .add_systems(
        Update,
        (
            update_mouse_deltas,
            window_focus_changed,
            apply_cursor_flags_on_change.in_set(CursorFlagsSet::UpdateCursorFlags),
        )
            .chain(),
    )
    .add_systems(OnEnter(GameState::Playing), create_debug);

    // This never matters
    app.allow_ambiguous_component::<Window>();
}
