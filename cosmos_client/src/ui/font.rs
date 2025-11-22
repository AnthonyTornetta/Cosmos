//! Contains fonts used by the game

use bevy::{
    app::Startup,
    asset::{AssetServer, Handle},
    prelude::{App, Commands, Deref, Res, Resource},
    text::Font,
};

#[derive(Resource, Deref)]
/// The default font used for most things
pub struct DefaultFont(pub Handle<Font>);

impl DefaultFont {
    /// Returns a weakly cloned handle to this font
    pub fn get(&self) -> Handle<Font> {
        self.0.clone()
    }
}

fn init_default_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(DefaultFont(asset_server.load("fonts/PixeloidSans.ttf")));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, init_default_font);
}
