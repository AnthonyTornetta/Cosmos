//! Represents all the energy stored on a structure

use bevy::prelude::*;

use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet},
    ecs::sets::FixedUpdateSet,
    entities::EntityId,
    events::block_events::BlockChangedMessage,
    faction::{FactionId, FactionRelation, Factions},
    physics::location::Location,
    prelude::{Ship, Station, StructureSystem},
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        events::StructureLoadedMessage,
        systems::{
            StructureSystemType, StructureSystems, StructureSystemsSet, SystemActive, SystemEnabled, WeaponSystem,
            dock_system::Docked,
            missile_launcher_system::PilotFocusing,
            turret_system::{TurretBlocks, TurretSystem, TurretTarget},
        },
    },
};

use crate::{
    ai::hit_tracking::Hitters,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    structure::systems::dock_system::DockedEntities,
};

use super::sync::register_structure_system;

fn register_turret_blocks(blocks: Res<Registry<Block>>, mut turret_blocks: ResMut<TurretBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:turret_base") {
        turret_blocks.insert(block);
    }
}

fn turret_block_update_system(
    mut event: MessageReader<BlockChangedMessage>,
    turret_blocks: Res<TurretBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut TurretSystem>,
    q_systems: Query<&StructureSystems>,
) {
    for ev in event.read() {
        let Ok(systems) = q_systems.get(ev.block.structure()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut system_query) else {
            continue;
        };

        if turret_blocks.is_turret(blocks.from_numeric_id(ev.old_block)) {
            system.block_removed(ev.block.coords());
        }

        if turret_blocks.is_turret(blocks.from_numeric_id(ev.new_block)) {
            system.block_added(ev.block.coords());
        }
    }
}

fn turret_structure_loaded_event_processor(
    mut event_reader: MessageReader<StructureLoadedMessage>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    turret_blocks: Res<TurretBlocks>,
    registry: Res<Registry<StructureSystemType>>,
    q_turret_system: Query<(), With<TurretSystem>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_turret_system).is_ok() {
                continue;
            }

            let mut system = TurretSystem::default();

            for block in structure.all_blocks_iter(false) {
                if turret_blocks.is_turret(structure.block_at(block, &blocks)) {
                    system.block_added(block);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

impl DefaultPersistentComponent for TurretSystem {}

fn look_at_turret_target(
    q_docked: Query<&Docked>,
    mut q_position: Query<(&Location, Option<&mut Velocity>, &GlobalTransform)>,
    q_turret_system: Query<(Entity, &ChildOf), (With<TurretSystem>, With<SystemEnabled>)>,
    q_target: Query<&TurretTarget>,
    q_structure: Query<&GlobalTransform, With<Structure>>,
    q_location: Query<&Location>,
) {
    const TURN_GAIN: f32 = 8.0;
    const MAX_ANGVEL: f32 = 1.0;

    for (_turret_ent, child_of) in q_turret_system.iter() {
        let Ok(turret_docked) = q_docked.get(child_of.parent()) else {
            continue;
        };

        let mut docked = turret_docked;
        while let Ok(d) = q_docked.get(docked.to) {
            docked = d;
        }

        let structure = docked.to;

        let Ok((loc, vel, g_trans)) = q_position.get_mut(child_of.parent()) else {
            continue;
        };

        let Some(mut vel) = vel else {
            continue;
        };

        let Some(target) = q_target.get(structure).ok() else {
            // No target - rotate toward home orientation
            if let Ok(parent_g_trans) = q_structure.get(structure) {
                let target_rot = parent_g_trans.rotation() * turret_docked.relative_rotation;
                let desired_dir = target_rot.mul_vec3(Vec3::NEG_Z).normalize();

                let current_forward = g_trans.rotation().mul_vec3(Vec3::NEG_Z).normalize();

                let axis = current_forward.cross(desired_dir);
                let dot = current_forward.dot(desired_dir).clamp(-1.0, 1.0);
                let angle = dot.acos();

                vel.angvel = if axis.length_squared() > 0.0001 {
                    axis.normalize() * (angle * TURN_GAIN).min(MAX_ANGVEL)
                } else {
                    Vec3::ZERO
                };
            } else {
                vel.angvel = Vec3::ZERO;
            }
            continue;
        };

        let target_loc = if let Ok(target_loc) = q_location.get(target.get()) {
            let mut target_loc = *target_loc;

            if let Ok(trans) = q_structure.get(target.get()) {
                target_loc = target_loc - (trans.rotation() * Vec3::splat(0.5));
            }

            target_loc
        } else {
            vel.angvel = Vec3::ZERO;
            continue;
        };

        let diff = (target_loc - *loc).absolute_coords_f32();

        if diff.length_squared() < 0.01 {
            vel.angvel = Vec3::ZERO;
            continue;
        }

        let desired_dir = diff.normalize();

        let current_forward = g_trans.rotation().mul_vec3(Vec3::NEG_Z).normalize();

        let axis = current_forward.cross(desired_dir);
        let dot = current_forward.dot(desired_dir).clamp(-1.0, 1.0);
        let angle = dot.acos();

        vel.angvel = if axis.length_squared() > 0.01 {
            axis.normalize() * (angle * TURN_GAIN).min(MAX_ANGVEL)
        } else {
            Vec3::ZERO
        };
    }
}

fn set_turret_target(
    mut commands: Commands,
    q_focusing: Query<(Entity, &PilotFocusing, &Location, Option<&FactionId>, Option<&TurretTarget>)>,
    q_targets: Query<(Entity, &Location, Option<&FactionId>, &EntityId), Or<(With<Ship>, With<Station>)>>,
    q_docked: Query<&Docked>,
    factions: Res<Factions>,
    q_hitters: Query<&Hitters>,
) {
    for (ship, pilot_focusing, my_loc, this_faction, tt) in &q_focusing {
        let mut topmost = ship;
        while let Ok(docked) = q_docked.get(topmost) {
            topmost = docked.to;
        }

        let my_hitters = q_hitters.get(topmost).ok();

        let mut best_target = None;

        if let Some(ent) = pilot_focusing.focusing {
            let can_target = q_targets
                .get(ent)
                .map(|(ent, _loc, other_faction, ent_id)| {
                    should_be_targetted(topmost, &factions, this_faction, other_faction, ent_id, &q_docked, ent, my_hitters)
                })
                .unwrap_or(false);

            if can_target {
                best_target = Some(ent);
            }
        }

        if best_target.is_none() {
            // find closest best target
            let min_best_target = q_targets
                .iter()
                .filter(|(_, loc, _, _)| loc.is_within(my_loc, 2000.0))
                .filter(|(ent, _loc, other_faction, ent_id)| {
                    should_be_targetted(
                        topmost,
                        &factions,
                        this_faction,
                        *other_faction,
                        ent_id,
                        &q_docked,
                        *ent,
                        my_hitters,
                    )
                })
                .min_by_key(|(_, loc, _, _)| loc.distance_sqrd(my_loc) as i32);

            best_target = min_best_target.map(|x| x.0)
        };

        if let Some(best_target) = best_target {
            if tt.map(|tt| tt.get() != best_target).unwrap_or(true) {
                commands.entity(ship).insert(TurretTarget::new(best_target));
            }
        } else if tt.is_some() {
            commands.entity(ship).remove::<TurretTarget>();
        }
    }
}

fn should_be_targetted(
    this_topmost_entity: Entity,
    factions: &Factions,
    this_faction: Option<&FactionId>,
    other_faction: Option<&FactionId>,
    other_ent_id: &EntityId,
    q_docked: &Query<&Docked>,
    target_ent: Entity,
    my_hitters: Option<&Hitters>,
) -> bool {
    let mut topmost = target_ent;
    while let Ok(docked) = q_docked.get(topmost) {
        topmost = docked.to;
    }

    if topmost == this_topmost_entity {
        return false;
    }

    if let Some(faction) = this_faction.and_then(|f| factions.from_id(f)) {
        let other_fac = other_faction.and_then(|f| factions.from_id(f));
        faction.relation_with_entity(other_ent_id, other_fac) == FactionRelation::Enemy
    } else {
        my_hitters.is_some_and(|h| h.get_number_of_hits(topmost) > 0)
    }
}

fn on_activate(q_activate: Query<(Entity, Has<SystemEnabled>), (With<TurretSystem>, Added<SystemActive>)>, mut commands: Commands) {
    for (ent, is_enabled) in q_activate.iter() {
        let mut ecmds = commands.entity(ent);

        if is_enabled {
            ecmds.remove::<SystemEnabled>();
        } else {
            ecmds.insert(SystemEnabled);
        }
    }
}

fn propagate_enabled(
    mut removed_enabled: RemovedComponents<SystemEnabled>,
    q_added: Query<Entity, Added<SystemEnabled>>,
    q_docked: Query<&DockedEntities>,
    q_systems: Query<&StructureSystems>,
    q_turret_system: Query<(Entity, Has<SystemEnabled>), With<TurretSystem>>,
    q_changed_activate: Query<(Entity, &StructureSystem, Has<SystemEnabled>), With<TurretSystem>>,
    mut commands: Commands,
) {
    let removed = removed_enabled.read().collect::<Vec<_>>();

    for (_, ss, is_enabled) in q_changed_activate
        .iter()
        .filter(|(e, _, _)| q_added.contains(*e) || removed.contains(e))
    {
        let structure = ss.structure_entity();

        propagate_turret_enabled(structure, &q_docked, &q_systems, &q_turret_system, &mut commands, is_enabled);
    }
}

fn propagate_turret_enabled(
    this_ent: Entity,
    q_docked: &Query<&DockedEntities>,
    q_systems: &Query<&StructureSystems>,
    q_turret_system: &Query<(Entity, Has<SystemEnabled>), With<TurretSystem>>,
    commands: &mut Commands,
    is_enabled: bool,
) {
    if let Ok(docked) = q_docked.get(this_ent) {
        for ent in docked.iter() {
            if let Ok(systems) = q_systems.get(ent)
                && let Ok((turret_ent, is_this_enabled)) = systems.query(q_turret_system)
            {
                if is_this_enabled && !is_enabled {
                    commands.entity(turret_ent).remove::<SystemEnabled>();
                } else if !is_this_enabled && is_enabled {
                    commands.entity(turret_ent).insert(SystemEnabled);
                }
            }

            propagate_turret_enabled(ent, q_docked, q_systems, q_turret_system, commands, is_enabled);
        }
    }
}

fn activate_systems(
    q_turret_system: Query<(&TurretSystem, &StructureSystem, Has<SystemEnabled>)>,
    q_systems: Query<(&StructureSystems, Has<TurretTarget>)>,
    q_weapon: Query<(Entity, Has<SystemActive>), With<WeaponSystem>>,
    mut commands: Commands,
) {
    for (ts, ss, enabled) in q_turret_system.iter() {
        if !ts.is_turret() {
            continue;
        }

        let Ok((systems, has_target)) = q_systems.get(ss.structure_entity()) else {
            continue;
        };

        for system in systems.all_activatable_systems() {
            let Ok((ent, is_active)) = q_weapon.get(system) else {
                continue;
            };

            let should_fire = has_target && enabled;

            if should_fire && !is_active {
                commands.entity(ent).insert(SystemActive::Primary);
            } else if !should_fire && is_active {
                commands.entity(ent).remove::<SystemActive>();
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<TurretSystem>(app);

    app.insert_resource(TurretBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_turret_blocks)
        .add_systems(
            FixedUpdate,
            (
                turret_structure_loaded_event_processor
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                turret_block_update_system
                    .in_set(BlockMessagesSet::ProcessMessages)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            (on_activate, propagate_enabled)
                .chain()
                .in_set(StructureSystemsSet::UpdateSystems)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            (set_turret_target, look_at_turret_target, activate_systems)
                .chain()
                .run_if(in_state(GameState::Playing))
                .in_set(FixedUpdateSet::PrePhysics),
        )
        .register_type::<TurretSystem>();

    register_structure_system::<TurretSystem>(app, true, "cosmos:turret_base");
}
