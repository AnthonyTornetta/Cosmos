use std::{collections::HashMap, fs};

use anyhow::{Error, bail};
use bevy::prelude::*;

use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    physics::location::{Location, Sector, SectorUnit},
    registry::{Registry, identifiable::Identifiable},
    structure::{
        Structure,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType},
        full_structure::FullStructure,
        ship::Ship,
    },
};
use mc_schem::schem;
use serde::{Deserialize, Serialize};

use crate::{commands::SendCommandMessageMessage, persistence::loading::NeedsBlueprintLoaded};

use super::super::prelude::*;

struct ConvertCommand {
    path: String,
    save_as: String,
}

impl CosmosCommandType for ConvertCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() < 2 {
            return Err(ArgumentError::TooFewArguments);
        } else if ev.args.len() > 2 {
            return Err(ArgumentError::TooManyArguments);
        }

        let path = ev.args[0].clone();
        let save_as = ev.args[1].clone();

        if save_as.contains("/") || save_as.contains("\\") {
            return Err(ArgumentError::InvalidType {
                arg_index: 1,
                type_name: "File Path".into(),
            });
        }

        Ok(ConvertCommand { path, save_as })
    }
}

#[derive(Serialize, Deserialize)]
struct Mappings {
    mappings: HashMap<String, String>,
}

fn convert(command: &ConvertCommand, blocks: &Registry<Block>) -> Result<(), Error> {
    let from = &command.path;
    let to = &command.save_as;

    let (schematic, metadata) = match schem::Schematic::from_file(from) {
        Err(e) => {
            return Err(e.into());
        }
        Ok(d) => d,
    };

    info!("{:?}", metadata);

    let pallette_mapping = toml::from_str::<Mappings>(&fs::read_to_string("importing/schematic_mapping.toml").unwrap()).unwrap();

    let mut all_blocks = vec![];

    // let mut min = BlockCoordinate::new(CoordinateType::MAX, CoordinateType::MAX, CoordinateType::MAX);
    // let mut max = BlockCoordinate::new(CoordinateType::MIN, CoordinateType::MIN, CoordinateType::MIN);

    let mut core_pos = None;

    for region in schematic.regions {
        // yzx
        let [sy, sz, sx] = region.shape_yzx();
        let sy = sy as usize;
        let sz = sz as usize;
        let sx = sx as usize;

        let offset = region.offset;
        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let Some(block) = region.array_yzx.get((y, z, x)) else {
                        continue;
                    };
                    let block = &region.palette[(*block) as usize];
                    let name = format!("{}:{}", block.namespace, block.id);
                    info!("{:?}", block.attributes);

                    let Some(matched_block) = pallette_mapping.mappings.get(&name) else {
                        return Err(anyhow::Error::msg(format!("Missing matching block: {name}")));
                    };

                    let block = blocks.from_id(matched_block);

                    let Some(block) = block else {
                        return Err(anyhow::Error::msg(format!("Invalid block mapping {matched_block} doesn't exist!")));
                    };

                    let coord = BlockCoordinate::new(
                        (x as i32 + offset[0]) as CoordinateType,
                        (y as i32 + offset[1]) as CoordinateType,
                        (z as i32 + offset[2]) as CoordinateType,
                    );

                    if block.unlocalized_name() == "minecraft:waxed_weathered_cut_copper_stairs" {
                        core_pos = Some(coord);
                    }

                    all_blocks.push((block, coord));

                    // min.x = coord.x.min(min.x);
                    // min.y = coord.y.min(min.y);
                    // min.z = coord.z.min(min.z);
                    //
                    // max.x = coord.x.max(max.x);
                    // max.y = coord.y.max(max.y);
                    // max.z = coord.z.max(max.z);
                }
            }
        }
    }

    let Some(core_pos) = core_pos else {
        return Err(anyhow::Error::msg("Missing core ;("));
    };

    // let bounds = BlockCoordinate::new(max.x - min.x, max.y - min.y, max.z - min.z);

    let mut fs = Structure::Full(FullStructure::new(ChunkCoordinate::new(10, 10, 10)));

    let core_coords = Ship::default_ship_core_coords(&fs);
    let offset = core_coords - core_pos;

    for (b, coord) in all_blocks {
        let Ok(coord) = BlockCoordinate::try_from(offset + coord) else {
            return Err(anyhow::Error::msg("This structure is too big!"));
        };

        if !fs.is_within_blocks(coord) {
            return Err(anyhow::Error::msg("This structure is too big!"));
        }

        fs.set_block_at(coord, b, Default::default(), blocks, None);
    }

    Ok(())
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<ConvertCommand, _>(
        ServerCommand::new(
            "cosmos:convert-blueprint",
            "[schem path] [blueprint path]",
            "Converts a schem file into a bp file (admin only)",
        ),
        app,
        |mut evr_load: MessageReader<CommandMessage<ConvertCommand>>,
         mut commands: Commands,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         blocks: Res<Registry<Block>>| {
            for ev in evr_load.read() {
                if let Err(e) = convert(&ev.command, &blocks) {
                    ev.sender.write(format!("{e:?}"), &mut evw_send_message);
                } else {
                    ev.sender.write(format!("Conversion successful!"), &mut evw_send_message);
                }
            }
        },
    );
}
