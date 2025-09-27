use std::time::Duration;

use bevy::{ecs::component::HookContext, platform::collections::HashMap, prelude::*, time::common_conditions::on_timer};
use bevy_rapier3d::prelude::{RigidBody, Velocity};
use cosmos_core::{
    block::{
        Block,
        block_direction::ALL_BLOCK_DIRECTIONS,
        block_events::{BlockEventsSet, BlockInteractEvent},
        blocks::AIR_BLOCK_ID,
        data::BlockData,
        multiblock::prelude::*,
    },
    ecs::NeedsDespawned,
    entities::player::Player,
    events::{
        block_events::{BlockChangedEvent, BlockDataSystemParams},
        structure::structure_event::StructureEventIterator,
    },
    inventory::Inventory,
    item::usable::blueprint::BlueprintItemData,
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
    },
    notifications::Notification,
    physics::location::{Location, SetPosition},
    prelude::{BlockCoordinate, ChunkCoordinate, FullStructure, Ship, Structure, StructureLoadingSet, StructureTypeSet},
    registry::{Registry, identifiable::Identifiable},
};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::{
    blocks::multiblock::shipyard::StructureBeingBuilt, persistence::loading::load_blueprint, structure::ship::loading::ShipNeedsCreated,
};

fn on_place_blocks_impacting_shipyard(
    mut evr_block_changed_event: EventReader<BlockChangedEvent>,
    // blocks: Res<Registry<Block>>,
    mut q_shipyards: Query<(&Shipyards, &mut Structure)>,
    q_shipyard: Query<(&Shipyard, Entity)>,
    mut q_block_data: Query<&mut BlockData>,
    mut bs_params: BlockDataSystemParams,
    q_has_data: Query<(), With<Shipyard>>,
) {
    for (structure, bce) in evr_block_changed_event.read().group_by_structure() {
        let Ok((shipyards, mut structure)) = q_shipyards.get_mut(structure) else {
            continue;
        };

        // Any block placed in a shipyard will invalidate it
        for bce in bce {
            for (_, ent) in shipyards
                .iter()
                .flat_map(|x| q_shipyard.get(x))
                .filter(|(s, _)| s.coordinate_within(bce.block.coords()))
            {
                let Ok(block_coords) = q_block_data.get(ent) else {
                    continue;
                };
                let block_coords = block_coords.identifier.block.coords();

                structure.remove_block_data::<Shipyard>(block_coords, &mut bs_params, &mut q_block_data, &q_has_data);
            }
        }
    }
}

#[derive(Error, Debug, Clone, Copy, Serialize, Deserialize, Display)]
enum ShipyardError {
    #[display("Controller Touching too many frames ({_0}/1)")]
    ControllerTouchingTooManyFrames(#[error(not(source))] BlockCoordinate),
    #[display("Frame is not clear of obstructions at {_0}")]
    FrameNotClear(#[error(not(source))] BlockCoordinate),
    #[display("Missing frames")]
    MissingFrames,
}

fn compute_shipyard(structure: &Structure, controller: BlockCoordinate, frame_id: u16) -> Result<Shipyard, ShipyardError> {
    let mut starting_frame_block = ALL_BLOCK_DIRECTIONS.iter().flat_map(|x| {
        BlockCoordinate::try_from(controller + x.to_coordinates())
            .ok()
            .filter(|c| structure.is_within_blocks(*c) && structure.block_id_at(*c) == frame_id)
    });

    let starting_frame_coord = match (starting_frame_block.next(), starting_frame_block.next()) {
        (Some(c), None) => c,
        (Some(_), Some(c)) => return Err(ShipyardError::ControllerTouchingTooManyFrames(c)),
        (None, _) => return Err(ShipyardError::MissingFrames),
    };

    let valid = check_is_valid_rectangle_outline_multiblock(structure, starting_frame_coord, &[frame_id], 5, usize::MAX);

    let bounds = match valid {
        Err(e) => match e {
            RectangleMultiblockError::InvalidMultiblock(s) => {
                error!("{s:?}");
                return Err(ShipyardError::MissingFrames);
            }
            // This shouldn't ever happen, but just in case
            RectangleMultiblockError::TooBig => {
                error!("Got a toobig error code - this shouldn't happen.");
                return Err(ShipyardError::MissingFrames);
            }
            RectangleMultiblockError::TooSmall => {
                error!("Too small!");
                return Err(ShipyardError::MissingFrames);
            }
        },
        Ok(bounds) => bounds,
    };

    if let Some(e) = bounds.check_walls_filled(
        structure,
        &[frame_id, AIR_BLOCK_ID],
        &mut [RectangleLimit {
            block: frame_id,
            amount: bounds.perimeter() as usize,
        }],
    ) {
        match e {
            RectangleMultiblockValidityError::BrokenLimit { block: _, coordinate } => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
            RectangleMultiblockValidityError::InvalidBlock(coordinate) => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
        }
    }

    if let Some(e) = bounds.check_inside_filled(structure, &[AIR_BLOCK_ID], &mut []) {
        match e {
            RectangleMultiblockValidityError::BrokenLimit { block: _, coordinate } => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
            RectangleMultiblockValidityError::InvalidBlock(coordinate) => {
                return Err(ShipyardError::FrameNotClear(coordinate));
            }
        }
    }

    Ok(Shipyard::new(bounds, controller))
}

fn interact_with_shipyard(
    mut q_structure: Query<&mut Structure>,
    q_shipyard: Query<&Shipyard>,
    mut evr_interact: EventReader<BlockInteractEvent>,
    blocks: Res<Registry<Block>>,
    mut bs_params: BlockDataSystemParams,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<Shipyard>>,
    mut nevw_open_ui: NettyEventWriter<ShowShipyardUi>,
    q_player: Query<&Player>,
) {
    for ev in evr_interact.read() {
        let Some(b) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(b.structure()) else {
            error!("No shipyard structure!");
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:shipyard_controller") else {
            error!("No shipyard controller block!");
            return;
        };

        if structure.block_id_at(b.coords()) != block.id() {
            continue;
        }

        if structure.query_block_data(b.coords(), &q_shipyard).is_some() {
            nevw_open_ui.write(ShowShipyardUi { shipyard_block: b }, player.client_id());
            continue;
        }

        let Some(frame) = blocks.from_id("cosmos:shipyard_frame") else {
            error!("No frame block!");
            return;
        };

        let shipyard = match compute_shipyard(&structure, b.coords(), frame.id()) {
            Err(e) => {
                error!("{e}");
                continue;
            }
            Ok(shipyard) => shipyard,
        };

        structure.insert_block_data(b.coords(), shipyard, &mut bs_params, &mut q_block_data, &q_has_data);

        nevw_open_ui.write(ShowShipyardUi { shipyard_block: b }, player.client_id());
    }
}

fn on_set_blueprint(
    mut nevr_set_shipyard_blueprint: EventReader<NettyEventReceived<SetShipyardBlueprint>>,
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_data: Query<(), With<ShipyardState>>,
    players: Res<ServerLobby>,
    q_player_inventory: Query<&Inventory, With<Player>>,
    q_blueprint_item_data: Query<&BlueprintItemData>,
    q_shipyard: Query<&Shipyard, Without<ShipyardState>>,
    mut commands: Commands,
    mut bs_params: BlockDataSystemParams,
    mut nevw_notification: NettyEventWriter<Notification>,
) {
    for ev in nevr_set_shipyard_blueprint.read() {
        let structure_ent = ev.shipyard_block.structure();
        let Ok(mut shipyard_structure) = q_structure.get_mut(structure_ent) else {
            continue;
        };
        let Some(shipyard) = shipyard_structure.query_block_data(ev.shipyard_block.coords(), &q_shipyard) else {
            error!("Invalid shipyard block given!");
            nevw_notification.write(Notification::error("This shipyard is already working!"), ev.client_id);
            continue;
        };

        let Some(Some(data)) = players.player_from_id(ev.client_id).map(|e| {
            q_player_inventory
                .get(e)
                .ok()
                .filter(|i| i.len() > ev.blueprint_slot as usize)
                .map(|i| i.query_itemstack_data(ev.blueprint_slot as usize, &q_blueprint_item_data))
                .flatten()
        }) else {
            error!("Invalid slot - {}", ev.blueprint_slot);
            continue;
        };

        let path = data.get_blueprint_path();
        let Ok(bp) = load_blueprint(&path) else {
            error!("Could not read blueprint @ {path}");
            nevw_notification.write(Notification::error("Unknown blueprint!"), ev.client_id);
            continue;
        };

        // 1. Load blueprint structure
        let Ok(mut structure) = bp.serialized_data().deserialize_data::<Structure>("cosmos:structure") else {
            error!("Could not load structure from blueprint!");
            nevw_notification.write(Notification::error("Invalid blueprint!"), ev.client_id);
            continue;
        };

        let Some(structure_bounds) = FullStructure::placed_block_bounds(&mut structure) else {
            continue;
        };
        let structure_size = BlockCoordinate::try_from(structure_bounds.1 - structure_bounds.0).unwrap();
        let midpoint =
            (structure.block_relative_position(structure_bounds.0) + structure.block_relative_position(structure_bounds.1)) / 2.0;

        let full_structure = match &structure {
            Structure::Full(f) => f,
            Structure::Dynamic(_) => {
                error!("Cannot load dynamic structure in shipyard!");
                continue;
            }
        };

        // 2. Validate blueprint size
        let bounds = shipyard.bounds();
        let shipyard_size = bounds.size();

        if shipyard_size.x - 1 <= structure_size.x {
            error!("Blueprint too big!");
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }
        if shipyard_size.y - 1 <= structure_size.y {
            error!("Blueprint too big!");
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }
        if shipyard_size.z - 1 <= structure_size.z {
            error!("Blueprint too big!");
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }

        let ship_origin = (shipyard_structure.block_relative_position(bounds.negative_coords)
            + shipyard_structure.block_relative_position(bounds.positive_coords))
            / 2.0
            - midpoint;

        let mut totals_count = HashMap::default();
        let blocks_todo = full_structure
            .all_blocks_iter(&structure, false)
            .map(|c| {
                let id = full_structure.block_id_at(c);
                let block_info = full_structure.block_info_at(c);
                *totals_count.entry(id).or_default() += 1;
                (c, id, block_info)
            })
            .collect::<Vec<_>>();

        // 3. Attach data to block

        let entity = commands
            .spawn((
                Name::new("Ship being built"),
                Velocity::default(),
                Ship,
                ShipNeedsCreated,
                Transform::from_translation(ship_origin),
                Location::default(),
                SetPosition::Location,
                Structure::Full(FullStructure::new(ChunkCoordinate::new(10, 10, 10))),
                ChildOf(structure_ent),
                RigidBody::Fixed,
                StructureBeingBuilt,
            ))
            .id();

        shipyard_structure.insert_block_data(
            ev.shipyard_block.coords(),
            ShipyardState::Building(ShipyardDoingBlueprint {
                blocks_todo,
                total_blocks_count: totals_count,
                creating: entity,
            }),
            &mut bs_params,
            &mut q_block_data,
            &q_has_data,
        );
    }
}

fn dont_move_being_built(q_being_built: Query<Entity, Added<StructureBeingBuilt>>, mut commands: Commands) {
    for ent in q_being_built.iter() {
        commands.entity(ent).insert((RigidBody::Fixed, Velocity::zero()));
    }
}

fn manage_shipyards(
    mut q_shipyard_state: Query<(Entity, &mut ShipyardState)>,
    mut commands: Commands,
    mut q_structure: Query<&mut Structure, With<Ship>>,
    blocks: Res<Registry<Block>>,
    mut evw_block_change: EventWriter<BlockChangedEvent>,
) {
    for (ent, mut state) in q_shipyard_state.iter_mut() {
        match state.as_mut() {
            ShipyardState::Paused(_) => {
                continue;
            }
            ShipyardState::Building(doing_bp) => {
                let Ok(mut structure) = q_structure.get_mut(doing_bp.creating) else {
                    continue;
                };

                let Some((coords, block, info)) = doing_bp.blocks_todo.pop() else {
                    info!("Done building ship!");
                    commands.entity(ent).remove::<ShipyardState>();
                    commands
                        .entity(doing_bp.creating)
                        .remove::<StructureBeingBuilt>()
                        .insert(RigidBody::Dynamic)
                        .remove_parent_in_place();
                    continue;
                };

                if let Some(count) = doing_bp.total_blocks_count.get_mut(&block) {
                    *count -= 1;
                    if *count == 0 {
                        doing_bp.total_blocks_count.remove(&block);
                    }
                }

                if structure.has_block_at(coords) {
                    continue;
                }

                let Some(block) = blocks.try_from_numeric_id(block) else {
                    error!("Missing block id {block}");
                    continue;
                };

                structure.set_block_and_info_at(coords, block, info, &blocks, Some(&mut evw_block_change));
            }
            ShipyardState::Deconstructing(ent) => {
                let Ok(mut structure) = q_structure.get_mut(*ent) else {
                    continue;
                };

                let mut itr = structure.all_blocks_iter(false);
                if let Some(mut coords) = itr.next() {
                    if structure.block_at(coords, &blocks).unlocalized_name() == "cosmos:ship_core" {
                        if let Some(next) = itr.next() {
                            coords = next;
                        } else {
                            commands.entity(*ent).insert(NeedsDespawned);
                            commands.entity(*ent).remove::<ShipyardState>();
                        }
                    }
                    structure.remove_block_at(coords, &blocks, Some(&mut evw_block_change));
                } else {
                    commands.entity(*ent).insert(NeedsDespawned);
                    commands.entity(*ent).remove::<ShipyardState>();
                }
            }
        }
    }
}

fn add_shipyard_state_hooks(world: &mut World) {
    world
        .register_component_hooks::<ShipyardState>()
        .on_remove(|mut world, HookContext { entity, .. }| {
            let state = world.get::<ShipyardState>(entity).expect("Impossible to fail");
            match state {
                ShipyardState::Building(d) | ShipyardState::Paused(d) => {
                    let creating = d.creating;
                    if let Ok(mut ecmds) = world.commands().get_entity(creating) {
                        ecmds
                            .remove::<StructureBeingBuilt>()
                            .insert(RigidBody::Dynamic)
                            .remove_parent_in_place();
                    }
                }
                ShipyardState::Deconstructing(_) => {}
            }
        });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (on_place_blocks_impacting_shipyard, interact_with_shipyard, dont_move_being_built)
            .chain()
            .in_set(BlockEventsSet::ProcessEvents),
    )
    .add_systems(
        FixedUpdate,
        (manage_shipyards.run_if(on_timer(Duration::from_millis(200))), on_set_blueprint)
            .chain()
            .in_set(StructureLoadingSet::LoadStructure)
            .in_set(StructureTypeSet::Ship)
            .ambiguous_with(StructureLoadingSet::LoadStructure),
    )
    .add_systems(Startup, add_shipyard_state_hooks);
}
