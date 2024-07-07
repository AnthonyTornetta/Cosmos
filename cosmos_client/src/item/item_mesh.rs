//! Mesh-creation logic for items

use bevy::{
    math::{Rect, Vec2, Vec3},
    render::{color::Color, mesh::Mesh},
};
use cosmos_core::utils::array_utils::{expand_2d, flatten_2d};

use crate::{
    asset::materials::MaterialDefinition,
    rendering::{CosmosMeshBuilder, MeshBuilder, MeshInformation},
};

/// Creates a mesh for an item based on its image data.
pub fn create_item_mesh(square_image_data: &[u8], item_id: u16, image_index: u32, mat: &MaterialDefinition, scale: f32) -> Mesh {
    // Data is assumed to be a square image
    let w = ((square_image_data.len() / 4) as f32).sqrt() as usize;
    let h = w;

    let mut pixels = vec![None; w * h];

    // let pixel_size = 1.0 / w as f32;
    let pixel_height = 1.0 / 16.0;

    for y in 0..h {
        for x in 0..w {
            let data_idx = flatten_2d(x * 4, y, w * 4);
            let rgba = &square_image_data[data_idx..(data_idx + 4)];
            let r = rgba[0];
            let g = rgba[1];
            let b = rgba[2];
            let a = rgba[3];

            if a == 0 {
                continue;
            }

            pixels[flatten_2d(x, y, w)] = Some(Color::rgba(
                r as f32 / u8::MAX as f32,
                g as f32 / u8::MAX as f32,
                b as f32 / u8::MAX as f32,
                a as f32 / u8::MAX as f32,
            ));
        }
    }

    let mut cmbuilder = CosmosMeshBuilder::default();

    let ph = pixel_height / 2.0;

    for (idx, _) in pixels.iter().enumerate().filter(|(_, x)| x.is_some()) {
        let (x, y) = expand_2d(idx, w);

        let min = Vec2::new(x as f32 / w as f32, y as f32 / h as f32);
        let max = Vec2::new((x + 1) as f32 / w as f32, (y + 1) as f32 / h as f32);

        let pmin = min - Vec2::new(0.5, 0.5);
        let pmax = max - Vec2::new(0.5, 0.5);

        let mut info = MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![
                [pmax.x, ph, pmin.y],
                [pmin.x, ph, pmin.y],
                [pmin.x, ph, pmax.y],
                [pmax.x, ph, pmax.y],
            ],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        };

        info.scale(Vec3::splat(scale));

        cmbuilder.add_mesh_information(
            &info,
            Vec3::ZERO,
            Rect::from_corners(min, max),
            image_index,
            mat.add_item_material_data(item_id, &info),
        );

        let mut info = MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![
                [pmax.x, -ph, pmax.y],
                [pmin.x, -ph, pmax.y],
                [pmin.x, -ph, pmin.y],
                [pmax.x, -ph, pmin.y],
            ],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        };

        info.scale(Vec3::splat(scale));

        cmbuilder.add_mesh_information(
            &info,
            Vec3::ZERO,
            Rect::from_corners(min, max),
            image_index,
            mat.add_item_material_data(item_id, &info),
        );

        // TODO: Add side meshes when needed
    }

    cmbuilder.build_mesh()
}

/*

egistry.insert_value(BlockMeshInformation::new_multi_face(
        "cosmos:base_block",
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![[0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5], [0.5, -0.5, 0.5]],
            normals: [[1.0, 0.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            positions: vec![[-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[-1.0, 0.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            positions: vec![[0.5, 0.5, -0.5], [-0.5, 0.5, -0.5], [-0.5, 0.5, 0.5], [0.5, 0.5, 0.5]],
            normals: [[0.0, 1.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            positions: vec![[0.5, -0.5, 0.5], [-0.5, -0.5, 0.5], [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5]],
            normals: [[0.0, -1.0, 0.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            positions: vec![[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]],
            normals: [[0.0, 0.0, 1.0]; 4].to_vec(),
        }
        .into(),
        MeshInformation {
            indices: vec![0, 1, 2, 2, 3, 0],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            positions: vec![[-0.5, 0.5, -0.5], [0.5, 0.5, -0.5], [0.5, -0.5, -0.5], [-0.5, -0.5, -0.5]],
            normals: [[0.0, 0.0, -1.0]; 4].to_vec(),
        }
        .into(),
    ));
*/
