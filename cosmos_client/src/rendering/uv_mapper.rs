use bevy::prelude::Vec2;

pub struct UVMapper {
    atlas_width: usize,
    atlas_height: usize,

    individual_width: usize,
    individual_height: usize,

    padding_x: usize,
    padding_y: usize
}

impl UVMapper {
    pub fn new(atlas_width: usize, atlas_height: usize,
               individual_width: usize, individual_height: usize,
               padding_x: usize, padding_y: usize) -> Self {
        Self {
            atlas_width,
            atlas_height,
            individual_width,
            individual_height,
            padding_x,
            padding_y,
        }
    }

    pub fn map(&self, image_index: usize) -> [Vec2; 2] {
        let image_size_y = 2 * self.padding_y + self.individual_height;
        let image_size_x = 2 * self.padding_x + self.individual_width;

        let how_many_in_row = self.atlas_width / image_size_x;

        let y = (image_index / how_many_in_row) * image_size_y + self.padding_y;
        let x = (image_index % how_many_in_row) * image_size_x + self.padding_x;

        let y_end = y + self.individual_height;
        let x_end = x + self.individual_width;

        [Vec2::new(x as f32 / self.atlas_width as f32, y as f32 / self.atlas_height as f32),
            Vec2::new(x_end as f32 / self.atlas_width as f32, y_end as f32 / self.atlas_height as f32)]
    }
}