//! Handles the logic behind the creation of a reactor multiblock

use std::{cell::RefCell, rc::Rc};

use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
        block_face::BlockFace,
        data::BlockData,
        multiblock::reactor::{Reactor, ReactorActive, ReactorBounds, ReactorPowerGenerationBlock, Reactors},
    },
    entities::player::Player,
    events::block_events::BlockDataSystemParams,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::{DEFAULT_MAX_STACK_SIZE, Item},
    netty::{NettyChannelServer, cosmos_encoder, server_reliable_messages::ServerReliableMessages},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate},
        systems::StructureSystemsSet,
    },
};

use crate::ai::AiControlled;

mod impls;

/// Represents the maximum dimensions of the reactor, including the reactor casing
const MAX_REACTOR_SIZE: CoordinateType = 11;

fn find_wall_coords(
    ub_controller_coords: UnboundBlockCoordinate,
    structure: &Structure,
    direction_a: UnboundBlockCoordinate,
    direction_b: UnboundBlockCoordinate,
    blocks: &Registry<Block>,
    valid_blocks: &[&Block],
) -> Option<(BlockCoordinate, BlockCoordinate)> {
    let mut width = 0;

    let mut found_coords = None;
    {
        let search_direction = direction_a;

        let mut check_coords = search_direction;
        for _ in 0..MAX_REACTOR_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            // structure.set_block_at(check_here, valid_blocks[1], Default::default(), blocks, Some(ev_writer));

            width += 1;

            let block_here = structure.block_at(check_here, blocks);

            if !valid_blocks.contains(&block_here) {
                found_coords = Some(
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(check_here) - search_direction)
                        .expect("This is guarenteed from previous logic to be within the structure"),
                );
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let left_wall_coords = found_coords?;

    let mut found_coords = None;
    {
        let search_direction = direction_b;

        let mut check_coords = search_direction;
        for _ in width..=MAX_REACTOR_SIZE {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            let block_here = structure.block_at(check_here, blocks);

            if !valid_blocks.contains(&block_here) {
                found_coords = Some(
                    BlockCoordinate::try_from(UnboundBlockCoordinate::from(check_here) - search_direction)
                        .expect("This is guarenteed from previous logic to be within the structure"),
                );
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let right_wall_coords = found_coords?;

    Some((left_wall_coords, right_wall_coords))
}

fn check_is_valid_multiblock(structure: &Structure, controller_coords: BlockCoordinate, blocks: &Registry<Block>) -> Option<ReactorBounds> {
    let valid_blocks = [
        blocks.from_id("cosmos:reactor_casing").expect("Missing reactor casing"),
        blocks.from_id("cosmos:reactor_window").expect("Missing cosmos:reactor_window"),
        blocks
            .from_id("cosmos:reactor_controller")
            .expect("Missing cosmos:reactor_controller"),
    ];

    let rotation = structure.block_rotation(controller_coords);

    let ub_controller_coords = UnboundBlockCoordinate::from(controller_coords);

    let mut found_coords = None;

    {
        let search_direction = rotation.direction_of(BlockFace::Back).to_coordinates();

        // Start 2 back to now allow a 2x2x2 reactor - minimum size is 3x3x3
        let mut check_coords = search_direction + search_direction;
        for _ in 0..MAX_REACTOR_SIZE - 2 {
            let Ok(check_here) = BlockCoordinate::try_from(check_coords + ub_controller_coords) else {
                return None;
            };

            let block_here = structure.block_at(check_here, blocks);

            if valid_blocks.contains(&block_here) {
                found_coords = Some(check_here);
                break;
            }

            check_coords = check_coords + search_direction;
        }
    }

    let back_wall_coords = found_coords?;

    let (left_wall_coords, right_wall_coords) = find_wall_coords(
        ub_controller_coords,
        structure,
        rotation.direction_of(BlockFace::Left).to_coordinates(),
        rotation.direction_of(BlockFace::Right).to_coordinates(),
        blocks,
        &valid_blocks,
    )?;

    let (down_wall_coords, up_wall_coords) = find_wall_coords(
        ub_controller_coords,
        structure,
        rotation.direction_of(BlockFace::Bottom).to_coordinates(),
        rotation.direction_of(BlockFace::Top).to_coordinates(),
        blocks,
        &valid_blocks,
    )?;

    Some(ReactorBounds {
        negative_coords: BlockCoordinate::new(
            controller_coords
                .x
                .min(back_wall_coords.x)
                .min(right_wall_coords.x)
                .min(left_wall_coords.x)
                .min(up_wall_coords.x)
                .min(down_wall_coords.x),
            controller_coords
                .y
                .min(back_wall_coords.y)
                .min(right_wall_coords.y)
                .min(left_wall_coords.y)
                .min(up_wall_coords.y)
                .min(down_wall_coords.y),
            controller_coords
                .z
                .min(back_wall_coords.z)
                .min(right_wall_coords.z)
                .min(left_wall_coords.z)
                .min(up_wall_coords.z)
                .min(down_wall_coords.z),
        ),
        positive_coords: BlockCoordinate::new(
            controller_coords
                .x
                .max(back_wall_coords.x)
                .max(right_wall_coords.x)
                .max(left_wall_coords.x)
                .max(up_wall_coords.x)
                .max(down_wall_coords.x),
            controller_coords
                .y
                .max(back_wall_coords.y)
                .max(right_wall_coords.y)
                .max(left_wall_coords.y)
                .max(up_wall_coords.y)
                .max(down_wall_coords.y),
            controller_coords
                .z
                .max(back_wall_coords.z)
                .max(right_wall_coords.z)
                .max(left_wall_coords.z)
                .max(up_wall_coords.z)
                .max(down_wall_coords.z),
        ),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReactorValidity {
    Valid,
    TooManyControllers(BlockCoordinate),
    MissingCasing(BlockCoordinate),
}

fn check_block(
    coords: BlockCoordinate,
    valid_blocks: &[&Block],
    controller_block: &Block,
    structure: &Structure,
    blocks: &Registry<Block>,
    controller_coords: &mut Option<BlockCoordinate>,
) -> ReactorValidity {
    let block_here = structure.block_at(coords, blocks);
    if controller_block == block_here {
        if let Some(controller_coords) = controller_coords {
            if *controller_coords != coords {
                return ReactorValidity::TooManyControllers(coords);
            }
        } else {
            *controller_coords = Some(coords);
        }
    } else if !valid_blocks.contains(&block_here) {
        return ReactorValidity::MissingCasing(coords);
    }

    ReactorValidity::Valid
}

fn check_valid(bounds: ReactorBounds, structure: &Structure, blocks: &Registry<Block>) -> ReactorValidity {
    let mut controller_location = None;

    let valid_blocks = [
        blocks.from_id("cosmos:reactor_casing").expect("Missing reactor casing"),
        blocks.from_id("cosmos:reactor_window").expect("missing cosmos:reactor_window"),
    ];

    let controller_block = blocks
        .from_id("cosmos:reactor_controller")
        .expect("Missing cosmos:reactor_controller");

    for z in bounds.negative_coords.z..=bounds.positive_coords.z {
        for y in bounds.negative_coords.y..=bounds.positive_coords.y {
            let res = check_block(
                BlockCoordinate::new(bounds.negative_coords.x, y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(bounds.positive_coords.x, y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    for y in bounds.negative_coords.y..=bounds.positive_coords.y {
        for x in bounds.negative_coords.x..=bounds.positive_coords.x {
            let res = check_block(
                BlockCoordinate::new(x, y, bounds.negative_coords.z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(x, y, bounds.positive_coords.z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    for z in bounds.negative_coords.z..=bounds.positive_coords.z {
        for x in bounds.negative_coords.x..=bounds.positive_coords.x {
            let res = check_block(
                BlockCoordinate::new(x, bounds.negative_coords.y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }

            let res = check_block(
                BlockCoordinate::new(x, bounds.positive_coords.y, z),
                &valid_blocks,
                controller_block,
                structure,
                blocks,
                &mut controller_location,
            );
            if res != ReactorValidity::Valid {
                return res;
            }
        }
    }

    ReactorValidity::Valid
}

fn create_reactor(
    structure: &Structure,
    blocks: &Registry<Block>,
    reactor_blocks: &Registry<ReactorPowerGenerationBlock>,
    bounds: ReactorBounds,
    controller: BlockCoordinate,
) -> Reactor {
    let mut power_per_second = 0.0;
    let mut fuel_consumption_percent = 0.0;

    for block in structure.block_iter(bounds.negative_coords.into(), bounds.positive_coords.into(), true) {
        let block = structure.block_at(block, blocks);

        if let Some(reactor_block) = reactor_blocks.for_block(block) {
            power_per_second += reactor_block.power_per_second();
            fuel_consumption_percent += 1.0;
        }
    }

    Reactor::new(controller, power_per_second, fuel_consumption_percent, bounds)
}

fn on_piloted_by_ai(
    blocks_registry: Res<Registry<Block>>,
    reactor_blocks: Res<Registry<ReactorPowerGenerationBlock>>,
    mut q_structure: Query<(&mut Structure, &mut Reactors), (With<AiControlled>, Or<(Added<AiControlled>, Added<Reactors>)>)>,
    bds_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<Reactor>>,
    q_has_active: Query<(), With<ReactorActive>>,
    mut q_inventory: Query<&mut Inventory>,
    items: Res<Registry<Item>>,
    mut commands: Commands,
    needs_data: Res<ItemShouldHaveData>,
) {
    let bds_params = Rc::new(RefCell::new(bds_params));

    for (mut structure, mut reactors) in q_structure.iter_mut() {
        let reactor_block = blocks_registry
            .from_id("cosmos:reactor_controller")
            .expect("Missing reactor controller block!");

        let blocks = structure
            .all_blocks_iter(false)
            .filter(|x| structure.block_id_at(*x) == reactor_block.id())
            .collect::<Vec<BlockCoordinate>>();

        let reactor_fuel_cell = items.from_id("cosmos:uranium_fuel_cell").expect("Missing uranium fuel cell");

        for block_here in blocks {
            if let Some(bounds) = check_is_valid_multiblock(&structure, block_here, &blocks_registry) {
                match check_valid(bounds, &structure, &blocks_registry) {
                    ReactorValidity::Valid => {
                        let reactor = create_reactor(&structure, &blocks_registry, &reactor_blocks, bounds, block_here);

                        structure.insert_block_data(block_here, reactor, &mut bds_params.borrow_mut(), &mut q_block_data, &q_has_data);
                        structure.insert_block_data(
                            block_here,
                            ReactorActive,
                            &mut bds_params.borrow_mut(),
                            &mut q_block_data,
                            &q_has_active,
                        );
                        if let Some(mut inv) = structure.query_block_data_mut(block_here, &mut q_inventory, bds_params.clone()) {
                            inv.insert_item_at(0, reactor_fuel_cell, DEFAULT_MAX_STACK_SIZE, &mut commands, &needs_data);
                        }

                        reactors.add_reactor_controller(block_here);
                    }
                    _ => {
                        warn!("Invalid reactor on AI-Controlled structure");
                    }
                }
            }
        }
    }
}

fn on_interact_reactor(
    mut structure_query: Query<(&mut Structure, &mut Reactors)>,
    blocks: Res<Registry<Block>>,
    reactor_blocks: Res<Registry<ReactorPowerGenerationBlock>>,
    mut interaction: EventReader<BlockInteractEvent>,
    mut server: ResMut<RenetServer>,
    player_query: Query<&Player>,
    mut bds_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<Reactor>>,
) {
    for ev in interaction.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok((mut structure, mut reactors)) = structure_query.get_mut(s_block.structure()) else {
            continue;
        };

        let block = structure.block_at(s_block.coords(), &blocks);

        if block.unlocalized_name() == "cosmos:reactor_controller" {
            if reactors.iter().any(|reactor_block| *reactor_block == s_block.coords()) {
                continue;
            }

            if let Some(bounds) = check_is_valid_multiblock(&structure, s_block.coords(), &blocks) {
                match check_valid(bounds, &structure, &blocks) {
                    ReactorValidity::MissingCasing(_) => {
                        let Ok(player) = player_query.get(ev.interactor) else {
                            continue;
                        };
                        server.send_message(
                            player.client_id(),
                            NettyChannelServer::Reliable,
                            cosmos_encoder::serialize(&ServerReliableMessages::InvalidReactor {
                                reason: "The reactor is missing required casing.".into(),
                            }),
                        );
                    }
                    ReactorValidity::TooManyControllers(_) => {
                        let Ok(player) = player_query.get(ev.interactor) else {
                            continue;
                        };
                        server.send_message(
                            player.client_id(),
                            NettyChannelServer::Reliable,
                            cosmos_encoder::serialize(&ServerReliableMessages::InvalidReactor {
                                reason: "The reactor can only have 1 controller.".into(),
                            }),
                        );
                    }
                    ReactorValidity::Valid => {
                        let reactor = create_reactor(&structure, &blocks, &reactor_blocks, bounds, s_block.coords());

                        structure.insert_block_data(s_block.coords(), reactor, &mut bds_params, &mut q_block_data, &q_has_data);

                        reactors.add_reactor_controller(s_block.coords());
                    }
                };
            } else {
                let Ok(player) = player_query.get(ev.interactor) else {
                    continue;
                };
                server.send_message(
                    player.client_id(),
                    NettyChannelServer::Reliable,
                    cosmos_encoder::serialize(&ServerReliableMessages::InvalidReactor {
                        reason: "Invalid bounds for the reactor - maximum of 11x11x11.".into(),
                    }),
                );
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    impls::register(app);

    app.add_systems(
        FixedUpdate,
        (
            on_piloted_by_ai.in_set(StructureSystemsSet::InitSystems),
            on_interact_reactor
                .in_set(BlockEventsSet::PostProcessEvents)
                .after(StructureSystemsSet::UpdateSystems),
        )
            // .in_set(NetworkingSystemsSet::Between)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
