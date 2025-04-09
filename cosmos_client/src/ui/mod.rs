//! Responsible for all the user interfaces the client can have

use bevy::{
    app::Update,
    ecs::{
        component::Component,
        schedule::{IntoSystemSetConfigs, SystemSet},
    },
    prelude::{App, Entity, Event, ImageNode, Text},
    reflect::Reflect,
    ui::{BackgroundColor, Node},
};

pub mod components;
pub mod crosshair;
pub mod debug_info_display;
mod focus_cam;
pub mod font;
pub mod hotbar;
mod hud;
pub mod item_renderer;
pub mod main_menu;
pub mod message;
pub mod pause;
pub mod reactivity;
pub mod settings;
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

#[derive(Component, Debug, PartialEq, Eq, Hash, Reflect)]
/// When you make a menu that can be closed via the `Escape`/pause menu key, add this component to it.
pub struct OpenMenu {
    level: u32,
    close_method: CloseMethod,
}

#[derive(Event, Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
/// An event that is fired when a menu is closed for [`CloseMethod::Custom`] menus.
pub struct CloseMenuEvent(pub Entity);

#[derive(Default, Debug, PartialEq, Eq, Hash, Clone, Copy, Reflect)]
/// How a menu should be closed
pub enum CloseMethod {
    #[default]
    /// The menu should be despawned via [`NeedsDespawned`].
    Despawn,
    /// The menu should be set to [`Visibility::Hidden`]
    Visibility,
    /// This menu cannot be automatically closed (eg via escape)
    Disabled,
    /// You will handle closing this menu yourself
    ///
    /// You need to respond to the [`CloseMenuEvent`].
    Custom,
}

impl OpenMenu {
    /// Creates an open menu with this "level" of being above every other menu.
    ///
    /// This doesn't effect rendering order, rather effects which menu the "Escape" button will target first.
    /// Menus of the same level will all be closed together. Each escape press will remove the highest-level group of menus.
    ///
    /// Typically, if you are constructing a heirarchy of windows, you should start your base window
    /// at 0 and count up from there.
    pub fn new(level: u32) -> Self {
        Self {
            level,
            close_method: Default::default(),
        }
    }

    /// Creates an open menu with this "level" of being above every other menu.
    /// The close_method determines the logic used to close this menu. See [`CloseMethod`].
    ///
    /// This doesn't effect rendering order, rather effects which menu the "Escape" button will target first.
    /// Menus of the same level will all be closed together. Each escape press will remove the highest-level group of menus.
    ///
    /// Typically, if you are constructing a heirarchy of windows, you should start your base window
    /// at 0 and count up from there.   
    pub fn with_close_method(level: u32, close_method: CloseMethod) -> Self {
        Self { level, close_method }
    }

    /// Sets the level for this menu
    pub fn set_level(&mut self, level: u32) {
        self.level = level;
    }

    /// Gets the level for this menu
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Gets the method that should be used to close this menu
    pub fn close_method(&self) -> &CloseMethod {
        &self.close_method
    }
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
    main_menu::register(app);
    hud::register(app);
    font::register(app);
    pause::register(app);
    settings::register(app);
    focus_cam::register(app);

    app.configure_sets(Update, (UiSystemSet::PreDoUi, UiSystemSet::DoUi, UiSystemSet::FinishUi).chain())
        .register_type::<OpenMenu>()
        .add_event::<CloseMenuEvent>();

    // These probably don't matter
    app.allow_ambiguous_component::<Text>();
    app.allow_ambiguous_component::<BackgroundColor>();
    app.allow_ambiguous_component::<Node>();
    app.allow_ambiguous_component::<ImageNode>();
}
