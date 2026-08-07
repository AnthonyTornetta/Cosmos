use std::collections::{BTreeMap, HashMap};
use std::fs;

use anyhow::{Error, bail};
use bevy::prelude::*;

use bevy::prelude::*;
use bevy_rapier3d::dynamics::{RigidBody, Velocity};
use cosmos_core::{
    block::{
        Block,
        block_direction::BlockDirection,
        block_face::BlockFace,
        block_rotation::{BlockRotation, BlockSubRotation},
    },
    ecs::NeedsDespawned,
    physics::location::{Location, Sector, SectorUnit, SetPosition},
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

use crate::{
    commands::SendCommandMessageMessage,
    persistence::{loading::NeedsBlueprintLoaded, saving::NeedsBlueprinted},
    structure::ship::loading::ShipNeedsCreated,
};

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

fn mc_facing_to_block_direction(facing: &str) -> BlockDirection {
    match facing {
        "north" => BlockDirection::NegZ,
        "south" => BlockDirection::PosZ,
        "west" => BlockDirection::NegX,
        "east" => BlockDirection::PosX,
        "up" => BlockDirection::PosY,
        "down" => BlockDirection::NegY,
        _ => BlockDirection::PosY,
    }
}

fn get_mc_rotation(attributes: &BTreeMap<String, String>) -> BlockRotation {
    if let Some(facing) = attributes.get("facing") {
        let direction = mc_facing_to_block_direction(facing);
        return BlockRotation::face_front(direction);
    }

    if let Some(axis) = attributes.get("axis") {
        match axis.as_str() {
            "x" => return BlockRotation::new(BlockFace::Right, BlockSubRotation::None),
            "z" => return BlockRotation::new(BlockFace::Front, BlockSubRotation::None),
            _ => return BlockRotation::default(),
        }
    }

    BlockRotation::default()
}

fn convert(command: &ConvertCommand, blocks: &Registry<Block>) -> Result<Structure, Error> {
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
                    let mc_block = &region.palette[(*block) as usize];
                    let name = format!("{}:{}", mc_block.namespace, mc_block.id);
                    let mc_attributes = &mc_block.attributes;
                    // info!("{:?}", mc_block.attributes);

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

                    if block.unlocalized_name() == "cosmos:ship_core" {
                        core_pos = Some(coord);
                    }

                    let rotation = get_mc_rotation(mc_attributes);

                    all_blocks.push((block, coord, rotation));

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

    let mut structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(10, 10, 10)));

    let core_coords = Ship::default_ship_core_coords(&structure);
    let offset = core_coords - core_pos;

    for (b, coord, rotation) in all_blocks {
        let Ok(coord) = BlockCoordinate::try_from(offset + coord) else {
            return Err(anyhow::Error::msg("This structure is too big!"));
        };

        if !structure.is_within_blocks(coord) {
            return Err(anyhow::Error::msg("This structure is too big!"));
        }

        structure.set_block_at(coord, b, rotation, blocks, None);
    }

    Ok(structure)
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
                match convert(&ev.command, &blocks) {
                    Err(e) => {
                        ev.sender.write(format!("{e:?}"), &mut evw_send_message);
                    }
                    Ok(s) => {
                        commands.spawn((
                            Name::new("Ship"),
                            Velocity::default(),
                            Ship::new_for_structure(&s),
                            Transform::default(),
                            Location::default(),
                            RigidBody::Dynamic,
                            ShipNeedsCreated { already_has_core: true },
                            s,
                            NeedsBlueprinted {
                                blueprint_name: ev.command.save_as.clone(),
                                name: ev.command.save_as.clone(),
                                blueprint_type: None,
                                override_path: None,
                            },
                            NeedsDespawned,
                        ));
                        ev.sender.write(format!("Conversion successful!"), &mut evw_send_message);
                    }
                }
            }
        },
    );
}
