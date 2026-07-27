use crate::{
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    structure::block_health::BlockHealthSet,
};
use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rapier3d::prelude::Velocity;
use bevy_renet::RenetServer;
use cosmos_core::{
    block::{Block, block_events::*, blocks::AIR_BLOCK_ID, data::BlockData},
    events::{
        block_events::{BlockChangedMessage, BlockChangedReason, BlockDataChangedMessage},
        cancellable::{Cancellable, CancellableMessage, CancellableMessageCmdImpl},
    },
    netty::{
        NettyChannelServer, cosmos_encoder,
        server_reliable_messages::{BlockChanged, BlocksChangedPacket, ServerReliableMessages},
        sync::IdentifiableComponent,
        system_sets::NetworkingSystemsSet,
    },
    prelude::Structure,
    state::GameState,
};
use cosmos_core::{
    blockitems::BlockItems,
    entities::player::creative::Creative,
    inventory::{
        Inventory,
        itemstack::{ItemShouldHaveData, ItemStackSystemSet},
    },
    item::{Item, physical_item::PhysicalItem},
    physics::location::{Location, SetPosition},
    registry::{Registry, identifiable::Identifiable},
    structure::{shared::build_mode::BuildMode, ship::pilot::Pilot},
};
use serde::{Deserialize, Serialize};

use super::drops::BlockDrops;

fn handle_block_changed_event(
    mut evr_block_changed_event: MessageReader<BlockChangedMessage>,
    mut evr_block_data_changed: MessageReader<BlockDataChangedMessage>,
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
) {
    let events_iter = evr_block_changed_event.read();
    let iter_len = events_iter.len();
    let mut map = HashMap::new();

    for ev in events_iter {
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: ev.new_block,
            block_info: ev.new_block_info,
        });
    }
    for ev in evr_block_data_changed.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };
        if !map.contains_key(&ev.block.structure()) {
            map.insert(ev.block.structure(), Vec::with_capacity(iter_len));
        }
        map.get_mut(&ev.block.structure()).expect("Set above").push(BlockChanged {
            coordinates: ev.block,
            block_id: structure.block_id_at(ev.block.coords()),
            block_info: structure.block_info_at(ev.block.coords()),
        });
    }

    for (entity, v) in map {
        server.broadcast_message(
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::BlockChange {
                structure_entity: entity,
                blocks_changed_packet: BlocksChangedPacket(v),
            }),
        );
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, Default)]
pub struct AutoInsertMinedItems;
impl IdentifiableComponent for AutoInsertMinedItems {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:no_auto_insert_mined_items"
    }
}
impl DefaultPersistentComponent for AutoInsertMinedItems {}

/// This system is horribly smelly, and should be refactored soon.
fn handle_block_break_events(
    mut q_structure: Query<(&mut Structure, &Location, &GlobalTransform, Option<&Velocity>)>,
    mut event_reader: MessageReader<Cancellable<BlockBreakMessage>>,
    blocks: Res<Registry<Block>>,
    items: Res<Registry<Item>>,
    block_items: Res<BlockItems>,
    mut inventory_query: Query<(&mut Inventory, Option<&BuildMode>, Option<&ChildOf>), Without<BlockData>>,
    mut event_writer: MessageWriter<BlockChangedMessage>,
    mut q_inventory_block_data: Query<(&BlockData, &mut Inventory), With<AutoInsertMinedItems>>,
    mut commands: Commands,
    has_data: Res<ItemShouldHaveData>,
    q_pilot: Query<&Pilot>,
    drops: Res<BlockDrops>,
) {
    for ev in event_reader.read().flatten() {
        // This is a temporary fix for mining lasers - eventually these items will have specified destinations,
        // but for now just throw them where ever there is space. This will get horribly laggy as there are more
        // structures in the game

        if q_structure.contains(ev.breaker) {
            let Ok((mut structure, _, _, _)) = q_structure.get_mut(ev.block.structure()) else {
                continue;
            };

            let coord = ev.block.coords();
            let block = structure.block_at(coord, &blocks);

            if block.id() == AIR_BLOCK_ID {
                continue;
            }

            let drop = drops.generate_drop_for(block, &items, &block_items, &mut rand::rng());

            if let Some(drop) = drop {
                let item = drop.item;
                let quantity = drop.quantity;

                let mut inserted = false;
                let mut leftover = quantity;

                for (_, mut inventory) in q_inventory_block_data
                    .iter_mut()
                    .filter(|(block_data, _)| block_data.identifier.block.structure() == ev.breaker)
                {
                    leftover = inventory.insert_item(item, quantity, &mut commands, &has_data).0;
                    if leftover == 0 {
                        inserted = true;
                        break;
                    }
                }

                // As a last resort, stick the item in the pilot's inventory
                //
                // If there is no more room after that, just don't spawn the item. I don't want to spawn
                // thousands of item entities that would mega lag the server + clients near it.
                if !inserted
                    && let Ok(pilot) = q_pilot.get(ev.breaker)
                    && let Ok((mut inventory, _, _)) = inventory_query.get_mut(pilot.entity)
                {
                    inventory.insert_item(item, leftover, &mut commands, &has_data);
                }
            } else {
                warn!("Missing item id for block {:?}", block);
            }

            structure.remove_block_at(coord, &blocks, Some((&mut event_writer, BlockChangedReason::Entity(ev.breaker))));
        } else if let Ok((mut inventory, _build_mode, _parent)) = inventory_query.get_mut(ev.breaker) {
            if let Ok((mut structure, s_loc, g_trans, velocity)) = q_structure.get_mut(ev.block.structure()) {
                let coord = ev.block.coords();
                let block = structure.block_at(coord, &blocks);

                if block.id() == AIR_BLOCK_ID {
                    continue;
                }

                let drop = drops.generate_drop_for(block, &items, &block_items, &mut rand::rng());

                if let Some(drop) = drop {
                    let item = drop.item;
                    let quantity = drop.quantity;

                    let (left_over, _) = inventory.insert_item(item, quantity, &mut commands, &has_data);

                    if left_over != 0 {
                        let structure_rot = Quat::from_affine3(&g_trans.affine());
                        let item_spawn_loc = *s_loc + structure_rot * structure.block_relative_position(coord);
                        let item_vel = velocity.copied().unwrap_or_default().linvel;

                        let dropped_item_entity = commands
                            .spawn((
                                PhysicalItem,
                                item_spawn_loc,
                                Transform::from_rotation(structure_rot),
                                SetPosition::Transform,
                                Velocity {
                                    linvel: item_vel
                                        + Vec3::new(
                                            rand::random::<f32>() - 0.5,
                                            rand::random::<f32>() - 0.5,
                                            rand::random::<f32>() - 0.5,
                                        ),
                                    angvel: Vec3::ZERO,
                                },
                            ))
                            .id();

                        let mut physical_item_inventory = Inventory::new("", 1, None, dropped_item_entity);
                        physical_item_inventory.insert_item(item, left_over, &mut commands, &has_data);
                        commands.entity(dropped_item_entity).insert(physical_item_inventory);
                    }
                }

                structure.remove_block_at(coord, &blocks, Some((&mut event_writer, BlockChangedReason::Entity(ev.breaker))));
            }
        } else {
            error!("Unknown breaker entity {:?} - logging components", ev.breaker);
            commands.entity(ev.breaker).log_components();
        }
    }
}

fn handle_block_place_events(
    mut query: Query<&mut Structure>,
    mut event_reader: MessageMutator<Cancellable<BlockPlaceMessage>>,
    mut event_writer: MessageWriter<BlockChangedMessage>,
    mut player_query: Query<(&mut Inventory, Option<&Creative>)>,
    items: Res<Registry<Item>>,
    blocks: Res<Registry<Block>>,
    block_items: Res<BlockItems>,
    mut commands: Commands,
) {
    for place_event in event_reader.read() {
        let Cancellable::Active(place_event_data) = place_event else {
            continue;
        };

        let Ok((mut inv, creative)) = player_query.get_mut(place_event_data.placer) else {
            continue;
        };

        // Even if nothing gets placed, the client will assume there was something placed.
        // Thus, we still want to update the client about their inventory to make sure their inventory is up-to-date.
        inv.set_changed();

        let Ok(mut structure) = query.get_mut(place_event_data.block.structure()) else {
            continue;
        };
        if !structure.is_within_blocks(place_event_data.block.coords()) {
            error!("Place event coords invalid!");
            continue;
        }
        let coords = place_event_data.block.coords();
        let block_up = place_event_data.block_rotation;

        let Some(is) = inv.itemstack_at(place_event_data.inventory_slot) else {
            break;
        };

        let item = items.from_numeric_id(is.item_id());

        let Some(block_id) = block_items.block_from_item(item) else {
            break;
        };

        let block = blocks.from_numeric_id(block_id);

        if structure.has_block_at(coords) && !structure.block_at(coords, &blocks).is_fluid() {
            continue;
        }

        if block_id != place_event_data.block_id {
            place_event.cancel();
            // May have run out of the item or it was swapped with something else (not really possible currently, but more checks never hurt anyone)
            break;
        }

        if creative.is_some() || inv.decrease_quantity_at(place_event_data.inventory_slot, 1, &mut commands) == 0 {
            structure.set_block_at(
                coords,
                block,
                block_up,
                &blocks,
                Some((&mut event_writer, BlockChangedReason::Entity(place_event_data.placer))),
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<AutoInsertMinedItems>(app);

    app.add_cancellable_message::<BlockBreakMessage>()
        .add_cancellable_message::<BlockPlaceMessage>()
        .add_cancellable_message::<BlockInteractMessage>()
        .add_systems(
            FixedUpdate,
            (handle_block_break_events, handle_block_place_events)
                .chain()
                .in_set(ItemStackSystemSet::CreateDataEntity)
                .in_set(BlockMessagesSet::ChangeBlocks),
        );

    app.add_systems(
        FixedUpdate,
        handle_block_changed_event
            .in_set(NetworkingSystemsSet::SyncComponents)
            .after(BlockHealthSet::ProcessHealthChanges)
            .run_if(in_state(GameState::Playing)),
    );
}
