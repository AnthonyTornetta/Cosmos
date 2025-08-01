use std::{cell::RefCell, rc::Rc};

use bevy::{platform::collections::HashSet, prelude::*};

use bevy_rapier3d::{
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::QueryFilter,
};
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet, block_face::BlockFace, data::BlockData},
    entities::player::Player,
    events::{
        block_events::{BlockChangedEvent, BlockDataSystemParams},
        structure::structure_event::StructureEventIterator,
    },
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    physics::location::{Location, LocationPhysicsSet, SECTOR_DIMENSIONS},
    prelude::StructureSystem,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
        chunk::ChunkEntity,
        coordinates::{BlockCoordinate, UnboundCoordinateType},
        events::StructureLoadedEvent,
        shields::Shield,
        systems::{
            StructureSystemType, StructureSystems, StructureSystemsSet, SystemActive,
            energy_storage_system::EnergyStorageSystem,
            railgun_system::{InvalidRailgunReason, RailgunBlock, RailgunFiredEvent, RailgunFiredInfo, RailgunSystem, RailgunSystemEntry},
        },
    },
    utils::ecs::MutOrMutRef,
};

use super::{shield_system::ShieldHitEvent, sync::register_structure_system};

fn compute_railguns(
    structure: &Structure,
    blocks: &Registry<Block>,
    railgun_spots: impl Iterator<Item = RailgunSystemEntry>,
) -> Vec<RailgunSystemEntry> {
    let mut touched: HashSet<BlockCoordinate> = HashSet::default();

    let mut railguns = vec![];

    for railgun_origin in railgun_spots {
        if structure.block_at(railgun_origin.origin, blocks).unlocalized_name() != "cosmos:railgun_launcher" {
            continue;
        }

        const RAILGUN_MIN_SIZE: u32 = 8;

        const CAPACITANCE_PER_MAGNET: u32 = 20_000;
        const CHARGE_PER_CHARGER: f32 = 2_000.0;
        const COOLING_PER_COOLER: f32 = 100.0;
        const HEAT_CAP_PER_MAGNET: u32 = 3_000;
        const HEAT_PER_FIRE_PER_MAGNET: u32 = 1_000;

        let dir = structure.block_rotation(railgun_origin.origin);
        let forward_offset = dir.direction_of(BlockFace::Front).to_coordinates();

        const MAGNET: &str = "cosmos:magnetic_rail";
        const AIR: &str = "cosmos:air";

        const CAPACITOR: &str = "cosmos:railgun_capacitor";
        const COOLING: &str = "cosmos:cooling_mechanism";

        let rails = [
            (BlockFace::Left, MAGNET),
            (BlockFace::Right, MAGNET),
            (BlockFace::Top, MAGNET),
            (BlockFace::Bottom, MAGNET),
            (BlockFace::Front, AIR),
        ];

        // Railgun Daigram:
        //
        // `*` = Any Block
        // `O` = Air
        // `X` = Magnetic rail
        // `C` = Capacitor OR Coolant
        //
        // ```
        // ..C..
        // .CXC.
        // CXOXC
        // .CXC.
        // ..C..
        // ```

        let charge_or_cool_spots = [
            (BlockFace::Left, BlockFace::Left),
            (BlockFace::Left, BlockFace::Top),
            (BlockFace::Left, BlockFace::Bottom),
            (BlockFace::Right, BlockFace::Right),
            (BlockFace::Right, BlockFace::Top),
            (BlockFace::Right, BlockFace::Bottom),
            (BlockFace::Top, BlockFace::Top),
            (BlockFace::Bottom, BlockFace::Bottom),
        ];

        let mut railgun_length: u32 = 0;

        let mut obstruction = false;
        let mut touching = false;

        let mut n_chargers = 0;
        let mut n_coolers = 0;

        while rails.iter().copied().all(|(offset, block)| {
            let side_offset = dir.direction_of(offset).to_coordinates();

            BlockCoordinate::try_from(forward_offset * (railgun_length as UnboundCoordinateType) + railgun_origin.origin + side_offset)
                .map(|here| {
                    if !touched.insert(here) {
                        touching = true;
                        return false;
                    }

                    let valid = structure.block_at(here, blocks).unlocalized_name() == block;

                    if !valid && block == AIR {
                        obstruction = true;
                    }

                    valid
                })
                .unwrap_or(false)
        }) {
            for (a, b) in charge_or_cool_spots.iter().copied() {
                let offset = dir.direction_of(a).to_coordinates() + dir.direction_of(b).to_coordinates();
                let Ok(bc) =
                    BlockCoordinate::try_from(forward_offset * (railgun_length as UnboundCoordinateType) + railgun_origin.origin + offset)
                else {
                    continue;
                };

                let block = structure.block_at(bc, blocks);
                if block.unlocalized_name() == COOLING {
                    if !touched.insert(bc) {
                        touching = true;
                        break;
                    }

                    n_coolers += 1;
                } else if block.unlocalized_name() == CAPACITOR {
                    if !touched.insert(bc) {
                        touching = true;
                        break;
                    }

                    n_chargers += 1;
                }
            }

            railgun_length += 1;
        }

        let n_magnets = railgun_length * 4;

        let mut railgun = RailgunSystemEntry {
            origin: railgun_origin.origin,
            length: railgun_length,
            direction: dir.direction_of(BlockFace::Front),
            capacitance: n_magnets * CAPACITANCE_PER_MAGNET,
            charge_rate: n_chargers as f32 * CHARGE_PER_CHARGER,
            max_heat: HEAT_CAP_PER_MAGNET * n_magnets,
            cooling_rate: COOLING_PER_COOLER * n_coolers as f32,
            heat_per_fire: HEAT_PER_FIRE_PER_MAGNET * n_magnets,
            invalid_reason: None,
        };

        if touching {
            railgun.invalid_reason = Some(InvalidRailgunReason::TouchingAnother);
            railguns.push(railgun);
            continue;
        }

        if obstruction {
            railgun.invalid_reason = Some(InvalidRailgunReason::Obstruction);
            railguns.push(railgun);
            continue;
        }

        if railgun_length < RAILGUN_MIN_SIZE {
            railgun.invalid_reason = Some(InvalidRailgunReason::NoMagnets);
            railguns.push(railgun);
            continue;
        }

        if n_coolers == 0 {
            railgun.invalid_reason = Some(InvalidRailgunReason::NoCooling);
            railguns.push(railgun);
            continue;
        }

        if n_chargers == 0 {
            railgun.invalid_reason = Some(InvalidRailgunReason::NoCapacitors);
            railguns.push(railgun);
            continue;
        }

        railguns.push(railgun);
    }

    railguns
}

fn block_update_system(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut RailgunSystem>,
    q_structure: Query<&Structure>,
    q_system: Query<&StructureSystem>,
    mut systems_query: Query<&mut StructureSystems>,
    registry: Res<Registry<StructureSystemType>>,
    mut commands: Commands,
) {
    for (structure, events) in evr_block_changed.read().group_by_structure() {
        let Ok(mut systems) = systems_query.get_mut(structure) else {
            continue;
        };

        let Ok(structure) = q_structure.get(structure) else {
            continue;
        };

        let mut new_system_if_needed = RailgunSystem::default();

        let railgun_system = systems
            .query_mut(&mut system_query)
            .map(|x| MutOrMutRef::from(x))
            .unwrap_or(MutOrMutRef::from(&mut new_system_if_needed));

        let railguns = compute_railguns(
            structure,
            &blocks,
            railgun_system
                .railguns
                .iter()
                .cloned()
                .chain(events.iter().map(|ev| RailgunSystemEntry {
                    origin: ev.block.coords(),
                    ..Default::default()
                })),
        );

        match railgun_system {
            MutOrMutRef::Mut(mut existing_system) => {
                if railguns.is_empty() {
                    let system = *systems.query(&q_system).expect("This should always exist on a StructureSystem");
                    systems.remove_system(&mut commands, &system, &registry);
                } else {
                    existing_system.railguns = railguns;
                }
            }
            MutOrMutRef::Ref(_) => {
                if !railguns.is_empty() {
                    systems.add_system(&mut commands, RailgunSystem::new(railguns), &registry);
                }
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = RailgunSystem::default();

            let Some(railgun_controller) = blocks.from_id("cosmos:railgun_launcher").map(|x| x.id()) else {
                return;
            };

            let railguns = compute_railguns(
                structure,
                &blocks,
                structure
                    .all_blocks_iter(false)
                    .filter(|x| structure.block_id_at(*x) == railgun_controller)
                    .map(|x| RailgunSystemEntry {
                        origin: x,
                        ..Default::default()
                    }),
            );

            if !railguns.is_empty() {
                system.railguns = railguns;

                systems.add_system(&mut commands, system, &registry);
            }
        }
    }
}

const RAILGUN_TRAVEL_DISTANCE: f32 = 2000.0;

fn on_active(
    context_access: ReadRapierContext,
    mut q_structure: Query<(&mut Structure, &GlobalTransform, &RapierContextEntityLink)>,
    q_active: Query<(&StructureSystem, &RailgunSystem), With<SystemActive>>,
    blocks: Res<Registry<Block>>,
    q_parent: Query<&ChildOf>,
    q_chunk_entity: Query<&ChunkEntity>,
    mut q_shield: Query<(Entity, &mut Shield, &GlobalTransform, &ChildOf, &RapierContextEntityLink)>,
    mut evw_take_damage: EventWriter<BlockTakeDamageEvent>,
    mut evw_block_destroyed: EventWriter<BlockDestroyedEvent>,
    q_players: Query<(&Player, &Location)>,
    q_locs: Query<&Location>,
    mut nevw_railgun_fired: NettyEventWriter<RailgunFiredEvent>,
    mut evw_shield_hit_event: EventWriter<ShieldHitEvent>,
    mut q_railgun_data: Query<&mut RailgunBlock>,
    bs_params: BlockDataSystemParams,
) {
    let bs_params = Rc::new(RefCell::new(bs_params));
    for (ss, railgun_system) in q_active.iter() {
        let mut fired = vec![];

        let Ok(structure_loc) = q_locs.get(ss.structure_entity()) else {
            continue;
        };
        for railgun_entry in railgun_system.railguns.iter() {
            let Ok((structure, g_trans, pw)) = q_structure.get(ss.structure_entity()) else {
                continue;
            };

            let railgun_block_coords = railgun_entry.origin;

            let Some(mut railgun_block) = structure.query_block_data_mut(railgun_block_coords, &mut q_railgun_data, bs_params.clone())
            else {
                error!("Desync between railgun and railgun block!");
                continue;
            };

            if railgun_block.get_unready_reason(railgun_entry).is_some() {
                continue;
            }

            let no_collide_entity = ss.structure_entity();

            let rel_pos = structure.block_relative_position(railgun_block_coords);
            let block_rotation = structure.block_rotation(railgun_block_coords);
            let docking_look_direction = block_rotation.direction_of(BlockFace::Front);
            let front_direction = docking_look_direction.as_vec3();

            let abs_block_pos = g_trans.transform_point(rel_pos);

            let my_rotation = Quat::from_affine3(&g_trans.affine());
            let ray_dir = my_rotation.mul_vec3(front_direction);

            let context = context_access.get(*pw);

            let mut need_checked = vec![];

            let mut structures = vec![];

            context.intersections_with_ray(
                abs_block_pos,
                ray_dir,
                RAILGUN_TRAVEL_DISTANCE,
                false,
                QueryFilter::predicate(QueryFilter::default(), &|entity| {
                    if no_collide_entity == entity {
                        false
                    } else if let Ok(parent) = q_parent.get(entity) {
                        parent.parent() != no_collide_entity
                    } else {
                        true
                    }
                }),
                |hit_entity, _| {
                    let Ok(structure_entity) = q_chunk_entity.get(hit_entity).map(|x| x.structure_entity) else {
                        return true;
                    };

                    if structures.contains(&structure_entity) {
                        return true;
                    }

                    structures.push(structure_entity);
                    true
                },
            );

            for structure_entity in structures {
                let Ok((hit_structure, hit_g_trans, _)) = q_structure.get(structure_entity) else {
                    continue;
                };

                let relative_ray_point = hit_g_trans.compute_matrix().inverse().transform_point3(abs_block_pos);

                let relative_ray_dir = hit_g_trans.rotation().inverse() * ray_dir;

                need_checked.append(
                    &mut hit_structure
                        .raycast_iter(relative_ray_point, relative_ray_dir, RAILGUN_TRAVEL_DISTANCE, false)
                        .map(|block| {
                            let relative_pos = hit_structure.block_relative_position(block);
                            (
                                (relative_pos - relative_ray_point).length_squared(),
                                block,
                                structure_entity,
                                relative_pos,
                                *hit_g_trans * relative_pos,
                            )
                        })
                        .collect::<Vec<_>>(),
                );
            }

            need_checked.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            let mut shields = q_shield
                .iter_mut()
                .filter(|(_, s, _, parent, rapier_link)| *rapier_link == pw && parent.parent() != ss.structure_entity() && s.is_enabled())
                .collect::<Vec<_>>();

            let mut strength = (railgun_entry.length as f32).powf(1.2) * 1000.0;

            let mut length = RAILGUN_TRAVEL_DISTANCE;

            for (_, block, structure_ent, relative_point, abs_hit) in need_checked.iter() {
                for (shield_entity, shield, shield_g_trans, _, _) in shields
                    .iter_mut()
                    .filter(|(_, s, g_trans, _, _)| (g_trans.translation() - *abs_hit).length_squared() <= s.radius * s.radius)
                {
                    let remaining_strength = shield.strength() - strength;
                    shield.take_damage(strength);

                    evw_shield_hit_event.write(ShieldHitEvent {
                        shield_entity: *shield_entity,
                        relative_position: shield_g_trans.rotation().inverse() * (abs_hit - shield_g_trans.translation()),
                    });

                    strength = remaining_strength;

                    if strength <= 0.0 {
                        break;
                    }
                }

                let Ok((mut structure, _, _)) = q_structure.get_mut(*structure_ent) else {
                    continue;
                };

                let cur_hp = structure.get_block_health(*block, &blocks);

                if strength <= 0.0 {
                    break;
                }
                structure.block_take_damage(
                    *block,
                    &blocks,
                    strength,
                    Some((&mut evw_take_damage, &mut evw_block_destroyed)),
                    Some(*structure_ent),
                );

                strength -= cur_hp;

                if strength <= 0.0 {
                    length = (*relative_point - abs_block_pos).length();
                    break;
                }
            }

            railgun_block.energy_stored = 0;
            railgun_block.heat += railgun_entry.heat_per_fire as f32;

            fired.push(RailgunFiredInfo {
                origin: railgun_entry.origin,
                length,
                direction: ray_dir,
            });
        }

        nevw_railgun_fired.write_to_many(
            RailgunFiredEvent {
                railguns: fired,
                structure: ss.structure_entity(),
            },
            q_players
                .iter()
                .filter(|x| x.1.distance_sqrd(structure_loc) < SECTOR_DIMENSIONS * SECTOR_DIMENSIONS)
                .map(|x| x.0.client_id()),
        );
    }
}

fn charge_and_cool_railguns(
    mut q_railguns: Query<(&mut RailgunBlock, &BlockData)>,
    q_railgun_system: Query<&RailgunSystem>,
    q_structure_systems: Query<&StructureSystems>,
    mut q_energy_system: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (mut railgun_block, block_data) in q_railguns.iter_mut() {
        let Ok(ss) = q_structure_systems.get(block_data.identifier.block.structure()) else {
            continue;
        };

        let Ok(mut ess) = ss.query_mut(&mut q_energy_system) else {
            continue;
        };

        let delta = time.delta_secs();

        let Ok(rgs) = ss.query(&q_railgun_system) else {
            continue;
        };

        let Some(railgun) = rgs.railguns.iter().find(|x| x.origin == block_data.identifier.block.coords()) else {
            continue;
        };

        if !railgun.is_valid_structure() {
            continue;
        }

        let charge_rate = (railgun.charge_rate * delta).min((railgun.capacitance - railgun_block.energy_stored) as f32);

        let uncharged = ess.decrease_energy(charge_rate);
        let amt_charged = charge_rate - uncharged;
        railgun_block.energy_stored += amt_charged.floor() as u32;
        railgun_block.energy_stored = railgun_block.energy_stored.min(railgun.capacitance);

        railgun_block.heat -= railgun.cooling_rate * delta;
        railgun_block.heat = railgun_block.heat.max(0.0);
    }
}

pub(super) fn register(app: &mut App) {
    register_structure_system::<RailgunSystem>(app, true, "cosmos:railgun_launcher");

    app.add_systems(
        Update,
        (
            structure_loaded_event
                .in_set(StructureSystemsSet::InitSystems)
                .ambiguous_with(StructureSystemsSet::InitSystems),
            block_update_system
                .in_set(BlockEventsSet::ProcessEvents)
                .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            (charge_and_cool_railguns, on_active)
                .chain()
                .in_set(StructureSystemsSet::UpdateSystemsBlocks)
                .in_set(NetworkingSystemsSet::Between)
                .after(LocationPhysicsSet::DoPhysics)
                .run_if(in_state(GameState::Playing)),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
