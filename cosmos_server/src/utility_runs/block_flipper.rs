use std::{ffi::OsStr, fs};

use bevy::{
    app::App,
    ecs::{schedule::OnEnter, system::Res},
};
use cosmos_core::{
    block::{Block, BlockFace, BlockRotation, BlockSubRotation},
    netty::cosmos_encoder,
    registry::Registry,
    structure::Structure,
};
use walkdir::WalkDir;

use crate::{persistence::SerializedData, state::GameState};

fn update_them(dir: &str, blocks: &Registry<Block>) {
    for file in WalkDir::new(&dir)
        .max_depth(100)
        .into_iter()
        .flatten()
        .filter(|x| x.file_type().is_file())
    {
        let path = file.path();

        if path.extension() != Some(OsStr::new("bp")) {
            continue;
        }

        let data = fs::read(path).unwrap_or_else(|e| panic!("{e}\nCounldnt' read {path:?}"));

        let mut sd: SerializedData =
            cosmos_encoder::deserialize(&data).unwrap_or_else(|e| panic!("{e}\nBlue print @ {path:?} not serialized properly"));

        let mut structure = sd
            .deserialize_data::<Structure>("cosmos:structure")
            .expect("Blueprint didn't contain structure?");

        // borrow checker moment
        let mut need_to_change = vec![];

        for block in structure.all_blocks_iter(false) {
            let coords = block.coords();
            let block = structure.block_at(coords, blocks);
            if block.should_face_front() {
                need_to_change.push(coords);
            }
        }

        for coords in need_to_change {
            let block = structure.block_at(coords, blocks).clone();

            structure.set_block_at(
                coords,
                &block,
                BlockRotation {
                    block_up: BlockFace::Top,
                    sub_rotation: BlockSubRotation::Flip,
                },
                blocks,
                None,
            );
        }

        sd.serialize_data("cosmos:structure", &structure);

        fs::write(path, cosmos_encoder::serialize(&sd)).unwrap_or_else(|e| panic!("{e}\nFailed to write {path:?}"));
    }
}

fn update_blueprints(blocks: Res<Registry<Block>>) {
    update_them("blueprints", &blocks);
    update_them("default_blueprints", &blocks);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), update_blueprints);
}
