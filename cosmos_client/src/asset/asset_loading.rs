use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use cosmos_core::loader::{AddLoadingEvent, DoneLoadingEvent, LoadingManager};

use crate::{rendering::uv_mapper::UVMapper, state::game_state::GameState};

use super::shaders::ArrayTextureMaterial;

enum AtlasName {
    Main,
}

struct LoadingAsset {
    atlas_name: AtlasName,
    handle: Handle<Image>,
}

struct AssetsDoneLoadingEvent;
struct AssetsLoadingID(usize);

struct AssetsLoading(Vec<LoadingAsset>);

pub struct MainAtlas {
    //handle: Handle<Image>,
    pub material: Handle<StandardMaterial>,
    pub uv_mapper: UVMapper,
}

fn setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut loading: ResMut<AssetsLoading>,
    mut loader: ResMut<LoadingManager>,
    mut start_writer: EventWriter<AddLoadingEvent>,
) {
    let main_atlas = server.load("images/atlas/main.png");

    loading.0.push(LoadingAsset {
        handle: main_atlas,
        atlas_name: AtlasName::Main,
    });

    commands.insert_resource(AssetsLoadingID(loader.register_loader(&mut start_writer)));
}

fn assets_done_loading(
    mut commands: Commands,
    event_listener: EventReader<AssetsDoneLoadingEvent>,
    loading_id: Option<Res<AssetsLoadingID>>,
    mut loader: ResMut<LoadingManager>,
    mut end_writer: EventWriter<DoneLoadingEvent>,
) {
    if loading_id.is_some() && !event_listener.is_empty() {
        loader.finish_loading((&loading_id).as_ref().unwrap().0, &mut end_writer);

        commands.remove_resource::<AssetsLoadingID>();
    }
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Option<ResMut<AssetsLoading>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: EventWriter<AssetsDoneLoadingEvent>,
) {
    if loading.is_none() {
        return;
    }

    use bevy::asset::LoadState;

    match server.get_group_load_state((&loading).as_ref().unwrap().0.iter().map(|h| h.handle.id)) {
        LoadState::Failed => {
            panic!("Failed to load asset!!");
        }
        LoadState::Loaded => {
            // all assets are now ready

            for asset in &(&loading).as_ref().unwrap().0 {
                match asset.atlas_name {
                    AtlasName::Main => {
                        const PADDING: u32 = 2;
                        const IMAGE_DIMENSIONS: u32 = 16;

                        let image = images.get(&asset.handle).unwrap();

                        let img_size = image.size();

                        let mut data: Vec<u8> = Vec::new();

                        let mut i = 0;

                        for y in 0..img_size.y as usize {
                            let mut n = match y % IMAGE_DIMENSIONS as usize == 0
                                || (y + 1) % IMAGE_DIMENSIONS as usize == 0
                            {
                                true => 1 + PADDING,
                                false => 1,
                            };

                            while n > 0 {
                                let og_i = i;

                                for x in 0..img_size.x as usize {
                                    if x % IMAGE_DIMENSIONS as usize == 0
                                        || (x + 1) % IMAGE_DIMENSIONS as usize == 0
                                    {
                                        for _ in 0..(PADDING + 1) {
                                            data.push(image.data[i]);
                                            data.push(image.data[i + 1]);
                                            data.push(image.data[i + 2]);
                                            data.push(image.data[i + 3]);
                                        }
                                    } else {
                                        data.push(image.data[i]);
                                        data.push(image.data[i + 1]);
                                        data.push(image.data[i + 2]);
                                        data.push(image.data[i + 3]);
                                    }

                                    i += 4;
                                }

                                n -= 1;

                                if n != 0 {
                                    i = og_i;
                                }
                            }
                        }

                        let y = img_size.y as u32;
                        let x = img_size.x as u32;

                        let height = (y / IMAGE_DIMENSIONS) * (IMAGE_DIMENSIONS + PADDING * 2);
                        let width = (x / IMAGE_DIMENSIONS) * (IMAGE_DIMENSIONS + PADDING * 2);

                        // debug save
                        // image::save_buffer(&Path::new("image.png"), data.as_slice(), width, height, image::ColorType::Rgba8);

                        let img = Image::new(
                            Extent3d {
                                height,
                                width,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            data,
                            TextureFormat::Rgba8UnormSrgb,
                        );

                        let handle = images.set(asset.handle.clone(), img);

                        let material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(handle.clone()),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: false,
                            metallic: 0.0,
                            reflectance: 0.0,

                            ..default()
                        });

                        commands.insert_resource(MainAtlas {
                            //handle,
                            material: material_handle,
                            uv_mapper: UVMapper::new(
                                width as usize,
                                height as usize,
                                IMAGE_DIMENSIONS as usize,
                                IMAGE_DIMENSIONS as usize,
                                PADDING as usize,
                                PADDING as usize,
                            ),
                        });
                    }
                }
            }

            // Clear out handles to avoid continually checking
            commands.remove_resource::<AssetsLoading>();

            // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)

            event_writer.send(AssetsDoneLoadingEvent);
        }
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

pub fn register(app: &mut App) {
    app.insert_resource(AssetsLoading { 0: Vec::new() })
        .add_event::<AssetsDoneLoadingEvent>()
        .add_system_set(SystemSet::on_enter(GameState::PostLoading).with_system(setup))
        .add_system_set(
            SystemSet::on_update(GameState::PostLoading)
                .with_system(check_assets_ready)
                .with_system(assets_done_loading),
        );
}
