//! Load a cubemap texture onto a cube like a skybox and cycle through different compressed texture formats

use bevy::{
    asset::LoadState,
    core_pipeline::Skybox,
    prelude::*,
    render::render_resource::{TextureViewDescriptor, TextureViewDimension},
};

use crate::state::game_state::GameState;

use super::MainMenuCamera;

/// Order from top to bottom:
/// Right, Left, Top, Bottom, Front, Back
const CUBEMAP: &str = "skybox/main-menu-panorama.png";

#[derive(Resource)]
struct Cubemap {
    is_loaded: bool,
    image_handle: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let skybox_handle = asset_server.load(CUBEMAP);

    commands.insert_resource(Cubemap {
        is_loaded: false,
        image_handle: skybox_handle,
    });
}

fn asset_loaded(asset_server: Res<AssetServer>, mut images: ResMut<Assets<Image>>, mut cubemap: ResMut<Cubemap>) {
    if !cubemap.is_loaded && asset_server.get_load_state(cubemap.image_handle.id()) == Some(LoadState::Loaded) {
        let image = images.get_mut(&cubemap.image_handle).unwrap();
        // NOTE: PNGs do not have any metadata that could indicate they contain a cubemap texture,
        // so they appear as one texture. The following code reconfigures the texture as necessary.
        if image.texture_descriptor.array_layer_count() == 1 {
            image.reinterpret_stacked_2d_as_array(image.texture_descriptor.size.height / image.texture_descriptor.size.width);
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..default()
            });
        }

        cubemap.is_loaded = true;
    }
}

fn on_enter_main_menu(cubemap: Res<Cubemap>, mut commands: Commands, query: Query<Entity, With<MainMenuCamera>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(Skybox {
            image: cubemap.image_handle.clone(),
            brightness: 1000.0,
        });
    }
}

fn on_add_main_menu_camera(cubemap: Res<Cubemap>, mut commands: Commands, query: Query<Entity, Added<MainMenuCamera>>) {
    for ent in query.iter() {
        commands.entity(ent).insert(Skybox {
            image: cubemap.image_handle.clone(),
            brightness: 1000.0,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app //.add_plugin(MaterialPlugin::<CubemapMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (asset_loaded, on_add_main_menu_camera)
                .chain()
                .run_if(in_state(GameState::MainMenu)),
        )
        .add_systems(OnEnter(GameState::MainMenu), on_enter_main_menu);
}
