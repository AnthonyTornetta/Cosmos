use bevy::{
    prelude::{Assets, Handle, Image},
    reflect::Reflect,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::TextureFormatPixelInfo,
    },
    utils::HashMap,
};

/// Similar to bevy's default texture atlas, but the order they are inserted matters and assumes every texture is the same size and a square.
///
/// If an image is > than the size, it is assumed to be an array of textures and will be treated as unique textures
#[derive(Reflect, Clone, Debug)]
pub struct SquareTextureAtlas {
    indices: HashMap<Handle<Image>, u32>,
    atlas_texture: Handle<Image>,
    width: u32,
    height: u32,
}

impl SquareTextureAtlas {
    pub fn get_texture_index(&self, handle: &Handle<Image>) -> Option<u32> {
        self.indices.get(handle).copied()
    }

    pub fn get_atlas_handle(&self) -> &Handle<Image> {
        &self.atlas_texture
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
}

/// Similar to bevy's default texture atlas, but the order they are inserted matters and assumes every texture is the same size and a square.
///
/// If an image is > than the size, it is assumed to be an array of textures and will be treated as unique textures
pub struct SquareTextureAtlasBuilder {
    images: Vec<Handle<Image>>,
    texture_dimensions: u32,
}

impl SquareTextureAtlasBuilder {
    pub fn new(texture_dimensions: u32) -> Self {
        Self {
            images: vec![],
            texture_dimensions,
        }
    }

    pub fn add_texture(&mut self, handle: Handle<Image>) {
        self.images.push(handle);
    }

    pub fn create_atlas(self, textures: &mut Assets<Image>) -> SquareTextureAtlas {
        let mut total_height = 0;

        let mut indices = HashMap::new();
        let mut current_index = 0;

        let images = self
            .images
            .iter()
            .map(|image_handle| {
                let image = textures.get(image_handle).expect("Given invalid image");
                total_height += image.size().y as u32;

                indices.insert(image_handle.clone_weak(), current_index);

                let img_ratio = image.size().y as f32 / self.texture_dimensions as f32;

                assert_eq!(
                    image.size().x as u32,
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

                current_index += image.size().y as u32 / self.texture_dimensions;

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
        );

        println!("Total height: {total_height}");

        let mut y = 0;

        for texture in images {
            println!("Y: {y}");
            println!("Y normalized: {y}");
            let next_y = y + self.texture_dimensions as usize * texture.size().y as usize * format_size;
            atlas_texture.data[y..next_y].copy_from_slice(&texture.data);
            y = next_y;
        }

        let (width, height) = (atlas_texture.size().x as u32, atlas_texture.size().y as u32);

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
