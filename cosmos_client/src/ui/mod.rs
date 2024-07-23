//! Responsible for all the user interfaces the client can have

use bevy::{
    app::Update,
    ecs::{
        component::Component,
        schedule::{IntoSystemSetConfigs, SystemSet},
    },
    prelude::App,
    text::Text,
    ui::{BackgroundColor, Style},
};

pub mod components;
pub mod crosshair;
pub mod debug_info_display;
pub mod hotbar;
mod hud;
pub mod item_renderer;
pub mod main_menu;
pub mod message;
pub mod reactivity;
pub mod ship_flight;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// All systems that handle GUI interactions should be in here
pub enum UiSystemSet {
    /// Handles the logic behind detecting changed react values
    PreDoUi,
    /// UI systems should do their work here
    DoUi,
    /// After this, all UI systems are finished
    FinishUi,
}

#[derive(Component)]
/// Append most UI nodes to this.
///
/// Note that UI nodes appended to this will display behind 3d block models. Use `UiTopRoot` to display past those.
pub struct UiRoot;

#[derive(Component)]
/// Append UI nodes you want to display in front of 3d-models to this.
///
/// If you're not dealing with 3d model weirdness, please prefer to use `UiRoot`.
pub struct UiTopRoot;

pub(super) fn register(app: &mut App) {
    crosshair::register(app);
    hotbar::register(app);
    debug_info_display::register(app);
    item_renderer::register(app);
    message::register(app);
    ship_flight::register(app);
    components::register(app);
    reactivity::register(app);
    main_menu::register(app);
    hud::register(app);

    app.configure_sets(Update, (UiSystemSet::PreDoUi, UiSystemSet::DoUi, UiSystemSet::FinishUi).chain());

    // These probably don't matter
    app.allow_ambiguous_component::<Text>();
    app.allow_ambiguous_component::<BackgroundColor>();
    app.allow_ambiguous_component::<Style>();
}
