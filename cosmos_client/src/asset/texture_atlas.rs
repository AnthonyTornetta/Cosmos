//! Similar to bevy's default texture atlas, but the order they are inserted matters and assumes every texture is the same size and a square.

use bevy::{
    image::TextureFormatPixelInfo,
    platform::collections::HashMap,
    prelude::{Assets, Handle, Image},
    reflect::Reflect,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};

/// Similar to bevy's default texture atlas, but the order they are inserted matters and assumes every texture is the same size and a square.
///
/// If an image is > than the `height` field, it is assumed to be an array of textures and will be treated as unique textures. This is useful
/// when you need textures to be next to each other in the atlas (such as for animated textures).
#[derive(Reflect, Clone, Debug, Default)]
pub struct SquareTextureAtlas {
    indices: HashMap<Handle<Image>, u32>,
    atlas_texture: Handle<Image>,
    width: u32,
    height: u32,
}

impl SquareTextureAtlas {
    /// Gets the texture index for a specific image if it exists in this atlas
    pub fn get_texture_index(&self, handle: &Handle<Image>) -> Option<u32> {
        self.indices.get(handle).copied()
    }

    /// Gets the handle to this atlas's image
    ///
    /// The image has already been interpreted as a stacked 2d array texture
    pub fn get_atlas_handle(&self) -> &Handle<Image> {
        &self.atlas_texture
    }

    /// Returns the image data for just this texture image as bytes.
    ///
    /// The bits are formatted in the U8rgba format. I think.
    pub fn get_sub_image_data(atlas_image: &Image, index: u32) -> &[u8] {
        &atlas_image.data.as_ref().expect("Texture data not init :(")[(index * atlas_image.width() * atlas_image.width() * 4) as usize
            ..(((1 + index) * atlas_image.width() * atlas_image.width() * 4) as usize)]
    }

    /// Do not rely on the internal image's width and height, use this instead.
    ///
    /// The atlas image's width
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Do not rely on the internal image's width and height, use this instead.
    ///
    /// The atlas image's height
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Each image will be `N`x`N` dimensions, where this function returns the `N`.
    pub fn individual_image_dimensions(&self) -> u32 {
        self.width
    }
}

/// Similar to bevy's default texture atlas, but the order they are inserted matters and assumes every texture is the same size and a square.
///
/// If an image is > than the size, it is assumed to be an array of textures and will be treated as unique textures
#[derive(Debug, Clone)]
pub struct SquareTextureAtlasBuilder {
    images: Vec<Handle<Image>>,
    /// The texture dimensions that this atlas builder is for
    ///
    /// Do not add textures that don't have this dimensions (based on image's width - not height)
    pub texture_dimensions: u32,
}

impl SquareTextureAtlasBuilder {
    /// Creates a new atlas builder
    ///
    /// All textures fed into this should have these dimensions or it will panic.
    /// Note that if a texture with a height that is a multiple of these dimensions is given, it will treat that single
    /// texture as multiple textures and add them in order next to each other in the atlas.
    pub fn new(texture_dimensions: u32) -> Self {
        Self {
            images: vec![],
            texture_dimensions,
        }
    }

    /// All textures fed into this should have these dimensions or it will panic*.
    /// *Note that if a texture with a height that is a multiple of these dimensions is given, it will treat that single
    /// texture as multiple textures and add them in order next to each other in the atlas.
    pub fn add_texture(&mut self, handle: Handle<Image>) {
        self.images.push(handle);
    }

    /// Turns all the images given to this into one big atlas image that is usable as an array texture.
    pub fn create_atlas(self, textures: &mut Assets<Image>) -> SquareTextureAtlas {
        let mut total_height = 0;

        let mut indices = HashMap::new();
        let mut current_index = 0;

        let images = self
            .images
            .iter()
            .map(|image_handle| {
                let image = textures.get(image_handle).expect("Given invalid image");
                total_height += image.size().y;

                indices.insert(image_handle.clone_weak(), current_index);

                let img_ratio = image.size().y as f32 / self.texture_dimensions as f32;

                assert_eq!(
                    { image.size().x },
                    self.texture_dimensions,
                    "Invalid image width -- {}. Must be exactly {}",
                    image.size().x,
                    self.texture_dimensions
                );

                assert_eq!(
                    img_ratio,
                    img_ratio.floor(),
                    "Invalid image height -- {}. Must be multiple of {}",
                    image.size().y,
                    self.texture_dimensions
                );

                current_index += image.size().y / self.texture_dimensions;

                image
            })
            .collect::<Vec<&Image>>();

        let format = TextureFormat::Rgba8UnormSrgb;

        let format_size = format.pixel_size();

        let mut atlas_texture = Image::new(
            Extent3d {
                width: self.texture_dimensions,
                height: total_height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0; format_size * (self.texture_dimensions * total_height) as usize],
            format,
            RenderAssetUsages::default(),
        );

        let mut y = 0;

        let data = atlas_texture.data.as_mut().expect("Pixel data not initialized?");
        for texture in images {
            let next_y = y + self.texture_dimensions as usize * texture.size().y as usize * format_size;
            data[y..next_y].copy_from_slice(&texture.data.as_ref().expect("Pixel data for individual texture not initialized?"));
            y = next_y;
        }

        let (width, height) = (atlas_texture.size().x, atlas_texture.size().y);

        atlas_texture.reinterpret_stacked_2d_as_array(total_height / self.texture_dimensions);

        let atlas_texture_handle = textures.add(atlas_texture);

        SquareTextureAtlas {
            atlas_texture: atlas_texture_handle,
            indices,
            width,
            height,
        }
    }
}
