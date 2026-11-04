//! Author-friendly 4x4 sheets for the sixteen connected-texture states.

use bevy::{
    prelude::Image,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use serde::{Deserialize, Serialize};

/// Packs connected textures into a single easier-to-edit texture
/// Format:
/// ```
/// 10 11  9  8
/// 14 15 13 12
///  6  7  5  4
///  2  3  1  0
/// ```
/// The upper-left 3x3 forms a rectangle, the right column a vertical strip,
/// the bottom row a horizontal strip, and the bottom-right cell isn't connected to anything.
pub const CONNECTED_SHEET_LAYOUT: [usize; 16] = [10, 11, 9, 8, 14, 15, 13, 12, 6, 7, 5, 4, 2, 3, 1, 0];

/// A PNG containing a 4x4 grid of square connected textures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedSheet {
    /// Namespaced texture name, using the same paths as `Single` / `Connected`.
    pub texture: String,
    /// Width and height of one cell in pixels (not the entire sheet).
    pub tile_size: u32,
}

impl ConnectedSheet {
    /// The path to this asset
    pub fn asset_path(&self, folder: &str) -> String {
        let (modid, name) = self
            .texture
            .split_once(':')
            .unwrap_or_else(|| panic!("Invalid connected sheet texture '{}': expected namespace:name", self.texture));
        format!("{modid}/images/{folder}/{name}.png")
    }

    /// Reject sizes that cannot form a nonempty, sixteen-layer texture strip.
    pub fn validate(&self) -> Result<(), String> {
        if self.tile_size == 0 || self.tile_size.checked_mul(16).is_none() {
            return Err(format!(
                "Connected sheet '{}' has invalid tile_size {}",
                self.texture, self.tile_size
            ));
        }
        Ok(())
    }

    /// Convert the source sheet to a vertical strip ordered by connection mask.
    ///
    /// The source image is left untouched. Alpha and all pixel bytes are retained.
    pub fn to_strip(&self, image: &Image) -> Result<Image, String> {
        self.validate()?;
        let edge = self.tile_size * 4;
        let descriptor = &image.texture_descriptor;
        if image.width() != edge
            || image.height() != edge
            || descriptor.dimension != TextureDimension::D2
            || descriptor.size.depth_or_array_layers != 1
            || descriptor.mip_level_count != 1
        {
            return Err(format!(
                "Connected sheet '{}' must be a {edge}x{edge} 2D image with one mip level (tile_size {}), got {:?}",
                self.texture, self.tile_size, descriptor.size
            ));
        }
        if !matches!(descriptor.format, TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm) {
            return Err(format!(
                "Connected sheet '{}' must use RGBA8 pixels, got {:?}",
                self.texture, descriptor.format
            ));
        }
        let bytes = (edge as usize)
            .checked_mul(edge as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| format!("Connected sheet '{}' is too large", self.texture))?;
        let source = image
            .data
            .as_ref()
            .filter(|data| data.len() == bytes)
            .ok_or_else(|| format!("Connected sheet '{}' has missing or invalid pixel data", self.texture))?;

        let tile = self.tile_size as usize;
        let row_bytes = tile * 4;
        let tile_bytes = row_bytes * tile;
        let mut pixels = vec![0; bytes];
        for (cell, mask) in CONNECTED_SHEET_LAYOUT.into_iter().enumerate() {
            for row in 0..tile {
                let src = ((cell / 4 * tile + row) * edge as usize + cell % 4 * tile) * 4;
                let dst = mask * tile_bytes + row * row_bytes;
                pixels[dst..dst + row_bytes].copy_from_slice(&source[src..src + row_bytes]);
            }
        }
        let mut strip = image.clone();
        strip.data = Some(pixels);
        strip.texture_descriptor.size = Extent3d {
            width: self.tile_size,
            height: self.tile_size * 16,
            depth_or_array_layers: 1,
        };
        Ok(strip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::texture_atlas::{SquareTextureAtlas, SquareTextureAtlasBuilder};
    use bevy::{asset::RenderAssetUsages, prelude::Assets};

    fn sheet_image(tile: u32) -> Image {
        let edge = tile * 4;
        let mut data = vec![0; (edge * edge * 4) as usize];
        for y in 0..edge {
            for x in 0..edge {
                let cell = (y / tile * 4 + x / tile) as usize;
                let at = ((y * edge + x) * 4) as usize;
                // Cell ID and local x/y expose wrong ordering, flips and strides.
                data[at..at + 4].copy_from_slice(&[cell as u8, (x % tile) as u8, (y % tile) as u8, (x + y) as u8]);
            }
        }
        Image::new(
            Extent3d {
                width: edge,
                height: edge,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    fn spec(tile_size: u32) -> ConnectedSheet {
        ConnectedSheet {
            texture: "cosmos:test".into(),
            tile_size,
        }
    }

    #[test]
    fn connected_sheet_preserves_pixels_and_orders_all_masks() {
        // Expected authoring cell for masks 0..15, independent of the pack loop.
        let cells = [15, 14, 12, 13, 11, 10, 8, 9, 3, 2, 0, 1, 7, 6, 4, 5];
        for tile in [1, 16, 32] {
            let image = sheet_image(tile);
            let before = image.data.clone();
            let strip = spec(tile).to_strip(&image).unwrap();
            assert_eq!((strip.width(), strip.height()), (tile, tile * 16));
            let data = strip.data.as_ref().unwrap();
            for (mask, cell) in cells.into_iter().enumerate() {
                for y in 0..tile {
                    for x in 0..tile {
                        let at = ((mask as u32 * tile * tile + y * tile + x) * 4) as usize;
                        let alpha = (cell % 4 * tile + x + cell / 4 * tile + y) as u8;
                        assert_eq!(&data[at..at + 4], &[cell as u8, x as u8, y as u8, alpha]);
                    }
                }
            }
            assert_eq!(image.data, before);
        }
    }

    #[test]
    fn connected_sheet_rejects_invalid_sources() {
        let image = sheet_image(32);
        assert!(spec(0).to_strip(&image).unwrap_err().contains("tile_size"));
        assert!(spec(u32::MAX).to_strip(&image).is_err());
        assert!(spec(16).to_strip(&image).unwrap_err().contains("64x64"));
        let mut bad = image.clone();
        bad.texture_descriptor.size.height -= 1;
        assert!(spec(32).to_strip(&bad).is_err());
        let mut bad = image.clone();
        bad.texture_descriptor.format = TextureFormat::R8Unorm;
        assert!(spec(32).to_strip(&bad).unwrap_err().contains("RGBA8"));
        let mut bad = image.clone();
        bad.data = None;
        assert!(spec(32).to_strip(&bad).is_err());
        let mut bad = image.clone();
        bad.data.as_mut().unwrap().pop();
        assert!(spec(32).to_strip(&bad).is_err());
    }

    #[test]
    fn connected_sheet_packs_beside_existing_tiles_and_animation_strips() {
        let mut images = Assets::<Image>::default();
        let solid = |height, value| {
            Image::new(
                Extent3d {
                    width: 32,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                vec![value; (32 * height * 4) as usize],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            )
        };
        let single = images.add(solid(32, 17));
        let animation = images.add(solid(64, 29));
        let strip = spec(32).to_strip(&sheet_image(32)).unwrap();
        let expected = strip.data.clone().unwrap();
        let sheet = images.add(strip);
        let following = images.add(solid(32, 43));
        let mut builder = SquareTextureAtlasBuilder::new(32);
        for handle in [&single, &animation, &sheet, &following] {
            builder.add_texture(handle.clone());
        }
        let atlas = builder.create_atlas(&mut images);
        assert_eq!(atlas.get_texture_index(&single), Some(0));
        assert_eq!(atlas.get_texture_index(&animation), Some(1));
        assert_eq!(atlas.get_texture_index(&sheet), Some(3));
        assert_eq!(atlas.get_texture_index(&following), Some(19));
        let packed = images.get(atlas.get_atlas_handle()).unwrap();
        assert_eq!(packed.texture_descriptor.size.depth_or_array_layers, 20);
        for mask in 0..16 {
            assert_eq!(
                SquareTextureAtlas::get_sub_image_data(packed, 3 + mask),
                &expected[mask as usize * 4096..(mask as usize + 1) * 4096],
            );
        }
    }
}
