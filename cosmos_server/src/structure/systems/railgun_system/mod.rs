use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer, utils::HashSet};

use bevy_rapier3d::{
    plugin::{RapierContextEntityLink, ReadRapierContext},
    prelude::{CollisionGroups, Group, QueryFilter},
};
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_events::BlockEventsSet, block_face::BlockFace, block_rotation::BlockRotation},
    events::block_events::BlockChangedEvent,
    netty::system_sets::NetworkingSystemsSet,
    physics::location::LocationPhysicsSet,
    prelude::StructureSystem,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::{
        Structure,
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
        chunk::ChunkEntity,
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        events::StructureLoadedEvent,
        shields::Shield,
        systems::{
            StructureSystemType, StructureSystems, StructureSystemsSet, SystemActive,
            line_system::{Line, LineBlocks, LineColorBlock, LineColorProperty, LineProperty, LinePropertyCalculator, LineSystem},
            railgun_system::{Railgun, RailgunSystem},
        },
    },
};

use super::{BlockStructureSystem, sync::register_structure_system};

fn compute_railguns(structure: &Structure, blocks: &Registry<Block>, railgun_spots: impl Iterator<Item = BlockCoordinate>) -> Vec<Railgun> {
    let mut touched: HashSet<BlockCoordinate> = HashSet::default();

    let mut railguns = vec![];

    for railgun_origin in railgun_spots {
        if structure.block_at(railgun_origin, blocks).unlocalized_name() != "cosmos:railgun_launcher" {
            continue;
        }

        let dir = structure.block_rotation(railgun_origin);
        let forward_offset = dir.direction_of(BlockFace::Front).to_coordinates();

        const MAGNET: &str = "cosmos:magnetic_rail";
        const AIR: &str = "cosmos:air";

        let rails = [
            (BlockFace::Left, MAGNET),
            (BlockFace::Right, MAGNET),
            (BlockFace::Top, MAGNET),
            (BlockFace::Bottom, MAGNET),
            (BlockFace::Front, AIR),
        ];

        let mut n: u32 = 0;

        let mut obstruction = false;
        let mut touching = false;

        while rails.iter().copied().all(|(offset, block)| {
            let side_offset = dir.direction_of(offset).to_coordinates();

            BlockCoordinate::try_from(forward_offset * (n as UnboundCoordinateType) + railgun_origin + side_offset)
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
            n += 1;
        }

        let mut railgun = Railgun {
            origin: railgun_origin,
            length: n,
            direction: dir.direction_of(BlockFace::Front),
            capacitance: 0,
            energy_stored: 0,
            valid: false,
        };

        if touching {
            warn!("This railgun is touching another - invalid!");
            railguns.push(railgun);
            continue;
        }

        if obstruction {
            warn!("There's an obstruction!");
            railguns.push(railgun);
            continue;
        }

        if n < 2 {
            railguns.push(railgun);
            warn!("Railgun too small!");
            continue;
        }

        railgun.valid = true;
        railguns.push(railgun);
    }

    railguns
}

fn block_update_system(
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut RailgunSystem>,
    q_structure: Query<&Structure>,
    systems_query: Query<&StructureSystems>,
) {
    // const RAILGUN_BLOCKS: [&str; 4] = [
    //     "cosmos:railgun_launcher",
    //     "cosmos:magnetic_rail",
    //     "cosmos:railgun_capacitor",
    //     "cosmos:cooling_mechanism",
    // ];

    for ev in evr_block_changed.read() {
        let Ok(systems) = systems_query.get(ev.block.structure()) else {
            continue;
        };

        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut system_query) else {
            continue;
        };

        // let old_block = blocks.from_numeric_id(ev.old_block);
        // let new_block = blocks.from_numeric_id(ev.new_block);

        // if RAILGUN_BLOCKS.contains(&old_block.unlocalized_name()) || RAILGUN_BLOCKS.contains(&new_block.unlocalized_name()) {
        let railguns = compute_railguns(
            structure,
            &blocks,
            system.railguns.iter().map(|r| r.origin).chain([ev.block.coords()].into_iter()),
        );
        info!("Railguns: {railguns:?}");

        system.railguns = railguns;
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

            let railguns = compute_railguns(structure, &blocks, structure.all_blocks_iter(false));
            info!("Railguns: {railguns:?}");

            system.railguns = railguns;

            systems.add_system(&mut commands, system, &registry);
        }
    }
}
//
// impl<T: LineProperty, S: LinePropertyCalculator<T>> BlockStructureSystem<T> for LineSystem<T, S> {
//     fn add_block(&mut self, block: BlockCoordinate, block_rotation: BlockRotation, prop: &T) {
//         let block_direction = block_rotation.direction_of(BlockFace::Front);
//
//         let mut found_line = None;
//         // If a structure has two lines like this: (XXXXX XXXXXX) and an X is placed
//         // in that space, then those two lines need to be linked toegether into one cannon.
//         //
//         // If this variable is ever Some index, then the found_line has to be linked with
//         // the line at this index.
//         let mut link_to = None;
//
//         for (i, line) in self.lines.iter_mut().filter(|x| x.direction == block_direction).enumerate() {
//             let delta = block_direction.to_coordinates();
//
//             let start: UnboundBlockCoordinate = line.start.into();
//
//             let block: UnboundBlockCoordinate = block.into();
//
//             // Block is before start
//             if start.x - delta.x == block.x && start.y - delta.y == block.y && start.z - delta.z == block.z {
//                 if found_line.is_some() {
//                     link_to = Some(i);
//                     break;
//                 } else {
//                     // This should always be >= 0 because a block cannot placed at negative coordinates
//                     line.start.x = (start.x - delta.x) as CoordinateType;
//                     line.start.y = (start.y - delta.y) as CoordinateType;
//                     line.start.z = (start.z - delta.z) as CoordinateType;
//                     line.len += 1;
//                     line.properties.insert(0, *prop);
//                     line.property = S::calculate_property(&line.properties);
//
//                     found_line = Some(i);
//                 }
//             }
//             // Block is after end
//             else if start.x + delta.x * (line.len as UnboundCoordinateType) == block.x
//                 && start.y + delta.y * (line.len as UnboundCoordinateType) == block.y
//                 && start.z + delta.z * (line.len as UnboundCoordinateType) == block.z
//             {
//                 if found_line.is_some() {
//                     link_to = Some(i);
//                     break;
//                 } else {
//                     line.len += 1;
//                     line.properties.push(*prop);
//                     line.property = S::calculate_property(&line.properties);
//
//                     found_line = Some(i);
//                 }
//             }
//         }
//
//         if let Some(l1_i) = found_line {
//             if let Some(l2_i) = link_to {
//                 let [l1, l2] = self
//                     .lines
//                     .get_disjoint_mut([l1_i, l2_i])
//                     .expect("From and to should never be the same");
//
//                 // Must use the one before the other in the line so the properties line up.
//                 if match l1.direction {
//                     BlockDirection::PosX => l1.start.x > l2.start.x,
//                     BlockDirection::NegX => l1.start.x < l2.start.x,
//                     BlockDirection::PosY => l1.start.y > l2.start.y,
//                     BlockDirection::NegY => l1.start.y < l2.start.y,
//                     BlockDirection::PosZ => l1.start.z > l2.start.z,
//                     BlockDirection::NegZ => l1.start.z < l2.start.z,
//                 } {
//                     std::mem::swap(l1, l2);
//                 }
//
//                 l1.len += l2.len;
//                 l1.power += l2.power;
//                 l1.active_blocks.append(&mut l2.active_blocks);
//
//                 l1.properties.append(&mut l2.properties);
//                 l1.property = S::calculate_property(&l1.properties);
//
//                 self.lines.swap_remove(l2_i);
//             }
//             return;
//         }
//
//         // If gotten here, no suitable line was found
//
//         let color = calculate_color_for_line(self, block, block_direction);
//
//         let properties = vec![*prop];
//         let property = S::calculate_property(&properties);
//
//         self.lines.push(Line {
//             start: block,
//             direction: block_direction,
//             len: 1,
//             properties,
//             property,
//             color,
//             active_blocks: vec![],
//             power: 0.0,
//         });
//     }
//
//     fn remove_block(&mut self, sb: BlockCoordinate) {
//         for (i, line) in self.lines.iter_mut().enumerate() {
//             line.mark_block_inactive(sb);
//
//             if line.start == sb {
//                 let (dx, dy, dz) = line.direction.to_i32_tuple();
//                 line.properties.remove(0);
//                 line.property = S::calculate_property(&line.properties);
//                 line.start.x = (line.start.x as i32 + dx) as CoordinateType;
//                 line.start.y = (line.start.y as i32 + dy) as CoordinateType;
//                 line.start.z = (line.start.z as i32 + dz) as CoordinateType;
//                 line.len -= 1;
//
//                 if line.len == 0 {
//                     self.lines.swap_remove(i);
//                 }
//                 return;
//             } else if line.end() == sb {
//                 line.properties.pop();
//                 line.property = S::calculate_property(&line.properties);
//                 line.len -= 1;
//                 if line.len == 0 {
//                     self.lines.swap_remove(i);
//                 }
//                 return;
//             } else if line.within(&sb) {
//                 let l1_len = match line.direction {
//                     BlockDirection::PosX => sb.x - line.start.x,
//                     BlockDirection::NegX => line.start.x - sb.x,
//                     BlockDirection::PosY => sb.y - line.start.y,
//                     BlockDirection::NegY => line.start.y - sb.y,
//                     BlockDirection::PosZ => sb.z - line.start.z,
//                     BlockDirection::NegZ => line.start.z - sb.z,
//                 };
//
//                 let l2_len = line.len as CoordinateType - l1_len - 1;
//
//                 let mut l1_props = Vec::with_capacity(l1_len as usize);
//                 let mut l2_props = Vec::with_capacity(l2_len as usize);
//
//                 let percent_power_l1 = l1_len as f32 / line.len as f32;
//                 let percent_power_l2 = l2_len as f32 / line.len as f32;
//
//                 for prop in line.properties.iter().take(l1_len as usize) {
//                     l1_props.push(*prop);
//                 }
//
//                 for prop in line.properties.iter().skip(l1_len as usize + 1) {
//                     l2_props.push(*prop);
//                 }
//
//                 let l1_property = S::calculate_property(&l1_props);
//
//                 // we are within a line, so split it into two seperate ones
//                 let mut l1 = Line {
//                     start: line.start,
//                     direction: line.direction,
//                     len: l1_len,
//                     properties: l1_props,
//                     property: l1_property,
//                     color: line.color,
//                     power: percent_power_l1 * line.power,
//                     active_blocks: vec![],
//                 };
//
//                 l1.active_blocks = line
//                     .active_blocks
//                     .iter()
//                     .filter(|x| l1.within(x))
//                     .copied()
//                     .collect::<Vec<BlockCoordinate>>();
//
//                 let (dx, dy, dz) = line.direction.to_i32_tuple();
//
//                 let dist = l1_len as i32 + 1;
//
//                 let l2_property = S::calculate_property(&l2_props);
//                 let mut l2 = Line {
//                     start: BlockCoordinate::new(
//                         (line.start.x as i32 + dx * dist) as CoordinateType,
//                         (line.start.y as i32 + dy * dist) as CoordinateType,
//                         (line.start.z as i32 + dz * dist) as CoordinateType,
//                     ),
//                     direction: line.direction,
//                     len: l2_len,
//                     properties: l2_props,
//                     property: l2_property,
//                     color: line.color,
//                     power: percent_power_l2 * line.power,
//                     active_blocks: vec![], // this will probably have to be calculated later.
//                 };
//
//                 l2.active_blocks = line
//                     .active_blocks
//                     .iter()
//                     .filter(|x| l2.within(x))
//                     .copied()
//                     .collect::<Vec<BlockCoordinate>>();
//
//                 self.lines[i] = l1;
//                 self.lines.push(l2);
//
//                 return;
//             }
//         }
//     }
// }
//
// fn is_in_line_with(testing_block: BlockCoordinate, direction: BlockDirection, line_coord: BlockCoordinate) -> bool {
//     match direction {
//         BlockDirection::PosX => line_coord.x >= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
//         BlockDirection::NegX => line_coord.x <= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
//         BlockDirection::PosY => line_coord.x == testing_block.x && line_coord.y >= testing_block.y && line_coord.z == testing_block.z,
//         BlockDirection::NegY => line_coord.x == testing_block.x && line_coord.y <= testing_block.y && line_coord.z == testing_block.z,
//         BlockDirection::PosZ => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z >= testing_block.z,
//         BlockDirection::NegZ => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z <= testing_block.z,
//     }
// }

const RAILGUN_TRAVEL_DISTANCE: f32 = 2000.0;

fn on_active(
    context_access: ReadRapierContext,
    mut q_structure: Query<(&mut Structure, &GlobalTransform, &RapierContextEntityLink)>,
    q_active: Query<(&StructureSystem, &RailgunSystem), With<SystemActive>>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_parent: Query<&Parent>,
    q_chunk_entity: Query<&ChunkEntity>,
    mut q_shield: Query<(&mut Shield, &GlobalTransform, &Parent, &RapierContextEntityLink)>,
    mut evw_take_damage: EventWriter<BlockTakeDamageEvent>,
    mut evw_block_destroyed: EventWriter<BlockDestroyedEvent>,
) {
    for (ss, ds) in q_active.iter() {
        for railgun in ds.railguns.iter().filter(|r| r.valid) {
            let Ok((structure, g_trans, pw)) = q_structure.get(ss.structure_entity()) else {
                continue;
            };

            let no_collide_entity = ss.structure_entity();

            let railgun_block = railgun.origin;
            let rel_pos = structure.block_relative_position(railgun_block);
            let block_rotation = structure.block_rotation(railgun_block);
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
                        parent.get() != no_collide_entity
                    } else {
                        true
                    }
                }),
                |hit_entity, intersection| {
                    let hit_point = abs_block_pos + ray_dir * intersection.time_of_impact;

                    info!("HIT ENTITY: {hit_entity:?}");

                    let Ok(structure_entity) = q_chunk_entity.get(hit_entity).map(|x| x.structure_entity) else {
                        return true;
                    };

                    if structures.iter().any(|(s_ent, _)| *s_ent == structure_entity) {
                        return true;
                    }

                    structures.push((structure_entity, intersection));
                    true
                },
            );

            for (structure_entity, intersection) in structures {
                let Ok((hit_structure, hit_g_trans, _)) = q_structure.get(structure_entity) else {
                    continue;
                };

                // let moved_point = intersection.time_of_impact * ray_dir - intersection.normal * 0.01;

                let point = hit_g_trans.compute_matrix().inverse().transform_point3(abs_block_pos);

                let moved_dir = hit_g_trans.rotation().inverse() * ray_dir;

                // let Ok(hit_coords) = hit_structure.relative_coords_to_local_coords_checked(point.x, point.y, point.z) else {
                //     return true;
                // };

                // need_checked.push((intersection.time_of_impact, hit_coords, structure_entity, moved_point));
                //
                info!("Hit point relative: {point:?}; Dir relative: {moved_dir:?}");

                need_checked.append(
                    &mut hit_structure
                        .raycast_iter(point, moved_dir, RAILGUN_TRAVEL_DISTANCE, false)
                        .map(|block| {
                            let relative_pos = hit_structure.block_relative_position(block);
                            (
                                (relative_pos - point).length() + intersection.time_of_impact,
                                block,
                                structure_entity,
                                relative_pos,
                            )
                        })
                        .collect::<Vec<_>>(),
                );
            }

            need_checked.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            info!("{need_checked:?}");

            let mut shields = q_shield
                .iter_mut()
                .filter(|(s, _, parent, rapier_link)| *rapier_link == pw && parent.get() != ss.structure_entity() && s.is_enabled())
                .collect::<Vec<_>>();

            let mut strength = 10000.0;

            for (_, block, structure_ent, point) in need_checked.iter() {
                for (shield, _, _, _) in shields
                    .iter_mut()
                    .filter(|(s, g_trans, _, _)| (g_trans.translation() - *point).length_squared() <= s.radius * s.radius)
                {
                    let remaining_strength = shield.strength() - strength;
                    shield.take_damage(strength);

                    strength = remaining_strength;

                    if strength <= 0.0 {
                        break;
                    }
                }

                let Ok((mut structure, _, _)) = q_structure.get_mut(*structure_ent) else {
                    continue;
                };

                let cur_hp = structure.get_block_health(*block, &blocks);

                structure.block_take_damage(
                    *block,
                    &blocks,
                    strength,
                    Some((&mut evw_take_damage, &mut evw_block_destroyed)),
                    Some(*structure_ent),
                );

                strength -= cur_hp;

                if strength <= 0.0 {
                    break;
                }
            }

            info!("Leftover: {strength}");
        }
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
            on_active
                .in_set(StructureSystemsSet::UpdateSystemsBlocks)
                .in_set(NetworkingSystemsSet::Between)
                .after(LocationPhysicsSet::DoPhysics)
                .run_if(in_state(GameState::Playing))
                .run_if(on_timer(Duration::from_secs(4))),
        )
            .run_if(in_state(GameState::Playing)),
    );
}
