use std::{cell::RefCell, rc::Rc, time::Duration};

use bevy::{ecs::component::HookContext, platform::collections::HashMap, prelude::*, time::common_conditions::on_timer};
use bevy_rapier3d::{
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::{Collider, QueryFilter, RigidBody, Velocity},
};
use cosmos_core::{
    block::{
        Block,
        block_direction::ALL_BLOCK_DIRECTIONS,
        block_events::{BlockEventsSet, BlockInteractEvent},
        blocks::AIR_BLOCK_ID,
        data::BlockData,
        multiblock::prelude::*,
    },
    blockitems::BlockItems,
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    entities::player::Player,
    events::{
        block_events::{BlockChangedEvent, BlockDataSystemParams},
        structure::structure_event::StructureEventIterator,
    },
    inventory::Inventory,
    item::{Item, usable::blueprint::BlueprintItemData},
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
    },
    notifications::Notification,
    physics::{
        location::{Location, SetPosition},
        structure_physics::ChunkPhysicsPart,
    },
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
    q_has_shipyard_data: Query<(), With<Shipyard>>,
    q_has_shipyard_state_data: Query<(), With<ShipyardState>>,
    q_has_shipyard_client_data: Query<(), With<ClientFriendlyShipyardState>>,
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

                structure.remove_block_data::<Shipyard>(block_coords, &mut bs_params, &mut q_block_data, &q_has_shipyard_data);
                structure.remove_block_data::<ShipyardState>(block_coords, &mut bs_params, &mut q_block_data, &q_has_shipyard_state_data);
                structure.remove_block_data::<ClientFriendlyShipyardState>(
                    block_coords,
                    &mut bs_params,
                    &mut q_block_data,
                    &q_has_shipyard_client_data,
                );
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
    mut nevw_notification: NettyEventWriter<Notification>,
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
                match e {
                    ShipyardError::MissingFrames => {
                        nevw_notification.write(
                            Notification::error("The shipyard is missing frames (min size 5x5x5)."),
                            player.client_id(),
                        );
                    }
                    ShipyardError::FrameNotClear(block) => {
                        nevw_notification.write(
                            Notification::error(format!("The shipyard is not clear of blocks. ({block})")),
                            player.client_id(),
                        );
                    }
                    ShipyardError::ControllerTouchingTooManyFrames(block) => {
                        nevw_notification.write(
                            Notification::error(format!("The controller can only be used for one shipyard. ({block})")),
                            player.client_id(),
                        );
                    }
                }

                continue;
            }
            Ok(shipyard) => shipyard,
        };

        structure.insert_block_data(b.coords(), shipyard, &mut bs_params, &mut q_block_data, &q_has_data);

        nevw_open_ui.write(ShowShipyardUi { shipyard_block: b }, player.client_id());
    }
}

fn on_set_blueprint(
    players: Res<ServerLobby>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    mut nevr_set_shipyard_blueprint: EventReader<NettyEventReceived<SetShipyardBlueprint>>,
    mut q_structure: Query<(&GlobalTransform, &mut Structure, &RapierContextEntityLink)>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_shipyard_data: Query<(), With<ShipyardState>>,
    mut q_inventory: Query<&mut Inventory, With<BlockData>>,
    (q_player_inventory, q_blueprint_item_data, q_shipyard, q_chunk_collider): (
        Query<&Inventory, (With<Player>, Without<BlockData>)>,
        Query<&BlueprintItemData>,
        Query<&Shipyard, Without<ShipyardState>>,
        Query<&ChunkPhysicsPart>,
    ),
    mut commands: Commands,
    bs_params: BlockDataSystemParams,
    mut nevw_notification: NettyEventWriter<Notification>,
    read_context: ReadRapierContext,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));

    for ev in nevr_set_shipyard_blueprint.read() {
        let structure_ent = ev.shipyard_block.structure();
        let Ok((station_g_trans, mut shipyard_structure, world)) = q_structure.get_mut(structure_ent) else {
            continue;
        };
        let Some(shipyard) = shipyard_structure.query_block_data(ev.shipyard_block.coords(), &q_shipyard) else {
            nevw_notification.write(Notification::error("This shipyard is already working!"), ev.client_id);
            continue;
        };

        let Some(ship_core_item) = items.from_id("cosmos:ship_core") else {
            continue;
        };

        let Some(ship_core_block) = blocks.from_id("cosmos:ship_core") else {
            continue;
        };

        let Some(Some(data)) = players.player_from_id(ev.client_id).map(|e| {
            q_player_inventory
                .get(e)
                .ok()
                .filter(|i| i.len() > ev.blueprint_slot as usize)
                .and_then(|i| i.query_itemstack_data(ev.blueprint_slot as usize, &q_blueprint_item_data))
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

        let bounds = shipyard.bounds();
        let shipyard_size = bounds.size();

        let half_size = Vec3::new(
            shipyard_size.x as f32 / 2.0,
            shipyard_size.y as f32 / 2.0,
            shipyard_size.z as f32 / 2.0,
        );

        let context = read_context.get(*world);

        let shipyard_world_pos = station_g_trans.translation()
            + station_g_trans.rotation() * (shipyard_structure.block_relative_position(shipyard.bounds().negative_coords) + half_size);

        info!("Checking {shipyard_world_pos}");

        let mut hit_something = false;

        context.intersections_with_shape(
            shipyard_world_pos,
            station_g_trans.rotation(),
            &Collider::cuboid(half_size.x, half_size.y, half_size.z),
            QueryFilter {
                exclude_rigid_body: Some(structure_ent),
                ..Default::default()
            },
            |e| {
                if let Ok(c) = q_chunk_collider.get(e)
                    && c.structure_entity == structure_ent
                {
                    return true;
                }

                hit_something = true;
                false
            },
        );

        if hit_something {
            nevw_notification.write(
                Notification::error("Please make sure shipyard is empty before starting!"),
                ev.client_id,
            );
            continue;
        }

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

        if shipyard_size.x - 1 <= structure_size.x {
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }
        if shipyard_size.y - 1 <= structure_size.y {
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }
        if shipyard_size.z - 1 <= structure_size.z {
            nevw_notification.write(Notification::error("Ship too big for this shipyard!"), ev.client_id);
            continue;
        }

        if !consume_item(
            &mut q_inventory,
            ev.shipyard_block.coords(),
            &shipyard_structure,
            ship_core_item,
            bs_params.clone(),
            &mut commands,
        ) {
            nevw_notification.write(
                Notification::error("No ship core in adjacent inventory to begin building ship!"),
                ev.client_id,
            );
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

        if let Some(entry) = totals_count.get_mut(&ship_core_block.id()) {
            *entry -= 1;
        }

        // 3. Attach data to block

        let entity = commands
            .spawn((
                Name::new("Ship being built"),
                Velocity::default(),
                Ship,
                ShipNeedsCreated,
                Transform::from_rotation(station_g_trans.rotation()),
                Location::default(),
                SetPosition::RelativeTo {
                    entity: structure_ent,
                    offset: ship_origin,
                },
                Structure::Full(FullStructure::new(ChunkCoordinate::new(10, 10, 10))),
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
            &mut bs_params.borrow_mut(),
            &mut q_block_data,
            &q_has_shipyard_data,
        );
    }
}

fn dont_move_being_built(q_being_built: Query<Entity, Added<StructureBeingBuilt>>, mut commands: Commands) {
    for ent in q_being_built.iter() {
        commands.entity(ent).insert((RigidBody::Fixed, Velocity::zero()));
    }
}

fn manage_shipyards(
    mut q_shipyard_state: Query<(Entity, &mut ShipyardState, &BlockData)>,
    mut commands: Commands,
    mut q_structure: Query<&mut Structure, (With<Ship>, With<StructureBeingBuilt>)>,
    q_building: Query<&Structure, Without<StructureBeingBuilt>>,
    blocks: Res<Registry<Block>>,
    mut evw_block_change: EventWriter<BlockChangedEvent>,
    bs_params: BlockDataSystemParams,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>,
    mut q_inventory: Query<&mut Inventory, With<BlockData>>,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));

    for (ent, mut state, block_data) in q_shipyard_state.iter_mut() {
        match state.as_mut() {
            ShipyardState::Paused(_) => {
                continue;
            }
            ShipyardState::Building(doing_bp) => {
                let Ok(mut structure) = q_structure.get_mut(doing_bp.creating) else {
                    continue;
                };

                let Ok(shipyard_structure) = q_building.get(block_data.structure()) else {
                    continue;
                };

                let Some((coords, block, info)) = doing_bp.blocks_todo.pop() else {
                    info!("Done building ship in shipyard!");
                    commands
                        .entity(ent)
                        .remove::<ShipyardState>()
                        .remove::<ClientFriendlyShipyardState>();
                    commands
                        .entity(doing_bp.creating)
                        .remove::<StructureBeingBuilt>()
                        .insert(RigidBody::Dynamic);
                    continue;
                };

                if let Some(count) = doing_bp.total_blocks_count.get_mut(&block) {
                    if *count != 0 {
                        *count -= 1;
                    }
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

                let Some(block_item) = block_items.item_from_block(block).map(|id| items.from_numeric_id(id)) else {
                    error!("Missing item for block {block:?}");
                    continue;
                };

                if !consume_item(
                    &mut q_inventory,
                    block_data.coords(),
                    shipyard_structure,
                    block_item,
                    bs_params.clone(),
                    &mut commands,
                ) {
                    doing_bp.blocks_todo.insert(0, (coords, block.id(), info));
                    if let Some(count) = doing_bp.total_blocks_count.get_mut(&block.id()) {
                        *count += 1;
                    } else {
                        doing_bp.total_blocks_count.insert(block.id(), 1);
                    }
                    continue;
                }

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
                        ecmds.remove::<StructureBeingBuilt>().insert(RigidBody::Dynamic);
                    }
                }
                ShipyardState::Deconstructing(_) => {}
            }
        });
}

fn on_change_shipyard_state(
    mut nevr_change_shipyard_state: EventReader<NettyEventReceived<ClientSetShipyardState>>,
    q_structure: Query<&Structure>,
    mut q_shipyard_state: Query<&mut ShipyardState>,
    bs_params: BlockDataSystemParams,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));
    for ev in nevr_change_shipyard_state.read() {
        let controller = ev.controller();
        let Ok(structure) = q_structure.get(controller.structure()) else {
            continue;
        };

        let Some(mut cur_state) = structure.query_block_data_mut(controller.coords(), &mut q_shipyard_state, bs_params.clone()) else {
            continue;
        };

        match &ev.event {
            ClientSetShipyardState::Deconstruct { controller: _ } => {
                error!("Not implemented yet!");
            }
            ClientSetShipyardState::Unpause { controller: _ } => {
                if let ShipyardState::Paused(d) = &**cur_state {
                    **cur_state = ShipyardState::Building(d.clone())
                }
            }
            ClientSetShipyardState::Pause { controller: _ } => {
                if let ShipyardState::Building(d) = &**cur_state {
                    **cur_state = ShipyardState::Paused(d.clone())
                }
            }
        }
    }
}

fn create_client_friendly_state(mut commands: Commands, q_state: Query<(Entity, &ShipyardState), Changed<ShipyardState>>) {
    for (ent, state) in q_state.iter() {
        commands.entity(ent).insert(state.as_client_friendly());
    }
}

fn consume_item(
    q_inventory: &mut Query<&mut Inventory, With<BlockData>>,
    center: BlockCoordinate,
    structure: &Structure,
    item: &Item,
    bs_params: Rc<RefCell<BlockDataSystemParams>>,
    commands: &mut Commands,
) -> bool {
    for dir in ALL_BLOCK_DIRECTIONS.iter() {
        let Ok(coord) = BlockCoordinate::try_from(dir.to_coordinates() + center) else {
            continue;
        };

        if !structure.is_within_blocks(coord) {
            continue;
        }

        if let Some(mut inv) = structure.query_block_data_mut(coord, q_inventory, bs_params.clone())
            && inv.take_and_remove_item(item, 1, commands).0 == 0
        {
            return true;
        }
    }
    false
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            on_place_blocks_impacting_shipyard,
            on_change_shipyard_state,
            interact_with_shipyard,
            dont_move_being_built,
            create_client_friendly_state,
        )
            .chain()
            .in_set(BlockEventsSet::ProcessEvents)
            .before(FixedUpdateSet::PrePhysics),
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
