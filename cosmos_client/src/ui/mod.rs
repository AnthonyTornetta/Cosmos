//! Responsible for all the user interfaces the client can have

use bevy::{
    app::Update,
    ecs::schedule::{apply_deferred, IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    prelude::App,
};

pub mod components;
pub mod crosshair;
pub mod debug_info_display;
pub mod hotbar;
pub mod item_renderer;
pub mod message;
pub mod reactivity;
mod ship_flight;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// All systems that handle GUI interactions should be in here
pub enum UiSystemSet {
    /// apply_deferred
    ApplyDeferredA,
    /// UI systems should do their work here
    DoUi,
    /// apply_deferred
    ApplyDeferredB,
    /// After this, all UI systems are finished
    FinishUi,
    /// apply_deferred
    ApplyDeferredC,
}

pub(super) fn register(app: &mut App) {
    crosshair::register(app);
    hotbar::register(app);
    debug_info_display::register(app);
    item_renderer::register(app);
    message::register(app);
    ship_flight::register(app);
    components::register(app);
    reactivity::register(app);

    app.configure_sets(
        Update,
        (
            UiSystemSet::ApplyDeferredA,
            UiSystemSet::DoUi,
            UiSystemSet::ApplyDeferredB,
            UiSystemSet::FinishUi,
            UiSystemSet::ApplyDeferredC,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            apply_deferred.in_set(UiSystemSet::ApplyDeferredA),
            apply_deferred.in_set(UiSystemSet::ApplyDeferredB),
            apply_deferred.in_set(UiSystemSet::ApplyDeferredC),
        ),
    );
}
