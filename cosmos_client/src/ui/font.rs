//! Contains fonts used by the game

use bevy::{
    app::Startup,
    asset::{AssetServer, Handle},
    prelude::{App, Commands, Res, Resource},
    text::Font,
};

#[derive(Resource)]
/// The default font used for most things
pub struct DefaultFont(pub Handle<Font>);

fn init_default_font(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(DefaultFont(asset_server.load("fonts/PixeloidSans.ttf")));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, init_default_font);
}
