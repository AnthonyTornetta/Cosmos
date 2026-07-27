use bevy::prelude::*;
use cosmos_core::{block::block_events::BlockPlaceMessage, structure::structure_block::StructureBlock};
use cosmos_core::{
    block::{
        Block,
        block_events::*,
        block_face::BlockFace,
        block_rotation::{BlockRotation, BlockSubRotation},
    },
    events::cancellable::Cancellable,
    prelude::Structure,
};
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundCoordinateType},
        shared::build_mode::{BuildAxis, BuildMode},
    },
};

/// Ensure we're not double-placing any blocks, which could happen if you place on the symmetry line
fn unique_push(vec: &mut Vec<(BlockCoordinate, BlockRotation)>, item: (BlockCoordinate, BlockRotation)) {
    for already_there in vec.iter() {
        if already_there.0 == item.0 {
            return;
        }
    }

    vec.push(item);
}

fn calculate_build_mode_blocks(
    mut structure_blocks: Vec<(BlockCoordinate, BlockRotation)>,
    build_mode: &BuildMode,
    parent: &ChildOf,
    structure_entity: Entity,
    structure: &Structure,
    block: &Block,
) -> Vec<(BlockCoordinate, BlockRotation)> {
    if parent.parent() != structure_entity {
        // Tried to place a block on a structure they're not in build mode on
        return vec![];
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::X) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_x_coord = 2 * (axis_coord - old_coords.x as UnboundCoordinateType) + old_coords.x as UnboundCoordinateType;
            if new_x_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(new_x_coord as CoordinateType, old_coords.y, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation,
                        },
                        BlockRotation {
                            face_pointing_pos_y: _,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Y) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_y_coord = 2 * (axis_coord - old_coords.y as UnboundCoordinateType) + old_coords.y as UnboundCoordinateType;
            if new_y_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, new_y_coord as CoordinateType, old_coords.z);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Top | BlockFace::Bottom,
                            sub_rotation: _,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Right | BlockFace::Left,
                            sub_rotation: BlockSubRotation::CCW | BlockSubRotation::CW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Back | BlockFace::Front,
                            sub_rotation: _,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        structure_blocks = new_coords;
    }

    if let Some(axis_coord) = build_mode.get_symmetry(BuildAxis::Z) {
        let axis_coord = axis_coord as UnboundCoordinateType;

        let mut new_coords = vec![];

        for (old_coords, block_rotation) in structure_blocks {
            unique_push(&mut new_coords, (old_coords, block_rotation));

            let new_z_coord = 2 * (axis_coord - old_coords.z as UnboundCoordinateType) + old_coords.z as UnboundCoordinateType;
            if new_z_coord >= 0 {
                let new_block_coords = BlockCoordinate::new(old_coords.x, old_coords.y, new_z_coord as CoordinateType);
                if structure.is_within_blocks(new_block_coords) {
                    let new_block_rotation = match block_rotation {
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Back | BlockFace::Front,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: BlockSubRotation::CW | BlockSubRotation::CCW,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y.inverse(),
                            sub_rotation: block_rotation.sub_rotation,
                        },
                        BlockRotation {
                            face_pointing_pos_y: BlockFace::Left | BlockFace::Right,
                            sub_rotation: _,
                        } => block_rotation.inverse(),
                        BlockRotation {
                            face_pointing_pos_y: _,
                            sub_rotation: BlockSubRotation::None | BlockSubRotation::Flip,
                        } => BlockRotation {
                            face_pointing_pos_y: block_rotation.face_pointing_pos_y,
                            sub_rotation: block_rotation.sub_rotation.inverse(),
                        },
                        _ => block_rotation,
                    };

                    unique_push(&mut new_coords, (new_block_coords, new_block_rotation));
                }
            }
        }

        // TODO: Sub rotations aren't properly handled by connected textures.
        // Nothing that has connected textures and uses sub rotations exists yet, so for now just
        // disable them on build block placements. This will have to get fixed eventually.
        let connected = !block.connect_to_groups.is_empty();
        if connected {
            for (_, rot) in &mut new_coords {
                rot.sub_rotation = Default::default();
            }
        }

        structure_blocks = new_coords;
    }

    structure_blocks
}

fn compute_build_mode_blocks(
    mut mr_place: MessageReader<Cancellable<BlockPlaceMessage>>,
    mut mr_break: MessageReader<Cancellable<BlockBreakMessage>>,
    q_build_mode: Query<(&BuildMode, &ChildOf)>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
) -> (Vec<Cancellable<BlockPlaceMessage>>, Vec<Cancellable<BlockBreakMessage>>) {
    let (mut new_place, mut new_break) = (vec![], vec![]);

    for ev in mr_place.read().flatten() {
        let Ok((build_mode, parent)) = q_build_mode.get(ev.placer) else {
            continue;
        };

        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        let mut structure_blocks = vec![(ev.block.coords(), BlockRotation::default())];

        let coord = ev.block.coords();
        let block = structure.block_at(coord, &blocks);

        structure_blocks = calculate_build_mode_blocks(structure_blocks, build_mode, parent, ev.block.structure(), structure, block);
        // the first block in this vec already has an event
        for (coord, rot) in structure_blocks.into_iter().skip(1) {
            new_place.push(Cancellable::from(BlockPlaceMessage {
                block_rotation: rot,
                block: StructureBlock::new(coord, ev.block.structure()),
                placer: ev.placer,
                block_id: ev.block_id,
                inventory_slot: ev.inventory_slot,
            }));
        }
    }

    for ev in mr_break.read().flatten() {
        let Ok((build_mode, parent)) = q_build_mode.get(ev.breaker) else {
            continue;
        };

        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        let mut structure_blocks = vec![(ev.block.coords(), BlockRotation::default())];

        let coord = ev.block.coords();
        let block = structure.block_at(coord, &blocks);

        structure_blocks = calculate_build_mode_blocks(structure_blocks, build_mode, parent, ev.block.structure(), structure, block);
        // the first block in this vec already has an event
        for (coord, _) in structure_blocks.into_iter().skip(1) {
            new_break.push(Cancellable::from(BlockBreakMessage {
                block: StructureBlock::new(coord, ev.block.structure()),
                breaker: ev.breaker,
                broken_id: structure.block_id_at(coord),
            }));
        }
    }

    (new_place, new_break)
}

fn send_events(
    input: In<(Vec<Cancellable<BlockPlaceMessage>>, Vec<Cancellable<BlockBreakMessage>>)>,
    mut mw_block_place: MessageWriter<Cancellable<BlockPlaceMessage>>,
    mut mw_block_break: MessageWriter<Cancellable<BlockBreakMessage>>,
) {
    let input = input.0;
    mw_block_place.write_batch(input.0);
    mw_block_break.write_batch(input.1);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        compute_build_mode_blocks
            .pipe(send_events)
            .in_set(BlockMessagesSet::PreRuleProcessing),
    );
}
