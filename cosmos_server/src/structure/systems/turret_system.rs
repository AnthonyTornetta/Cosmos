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
    prelude::{Ship, Station},
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        events::StructureLoadedMessage,
        ship::pilot::Pilot,
        systems::{
            StructureSystemType, StructureSystems, StructureSystemsSet,
            dock_system::Docked,
            missile_launcher_system::PilotFocusing,
            turret_system::{TurretBlocks, TurretSystem, TurretTarget},
        },
    },
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

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
    q_thruster_system: Query<(), With<TurretSystem>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_thruster_system).is_ok() {
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
    q_turret_system: Query<&ChildOf, With<TurretSystem>>,
    q_structure: Query<&TurretTarget>,
) {
    for child_of in q_turret_system.iter() {
        let Ok(mut docked) = q_docked.get(child_of.parent()) else {
            continue;
        };

        while let Ok(d) = q_docked.get(docked.to) {
            docked = d;
        }

        let structure = docked.to;

        let Ok(target) = q_structure.get(structure) else {
            info!("No target");
            continue;
        };

        let Ok((&target_loc, _, _)) = q_position.get(target.get()) else {
            info!("Target has bad stuff ;(");
            continue;
        };

        let Ok((loc, mut vel, g_trans)) = q_position.get_mut(child_of.parent()) else {
            info!("no loc");
            continue;
        };

        let Some(mut vel) = vel else {
            error!("no vel");
            continue;
        };

        let diff = (target_loc - *loc).absolute_coords_f32();

        if diff.length_squared() < 0.0001 {
            vel.angvel = Vec3::ZERO;
            continue;
        }

        let desired_dir = diff.normalize();

        let current_forward = g_trans.rotation().mul_vec3(Vec3::NEG_Z).normalize();

        let axis = current_forward.cross(desired_dir);
        let dot = current_forward.dot(desired_dir).clamp(-1.0, 1.0);
        let angle = dot.acos();

        const TURN_GAIN: f32 = 8.0;
        const MAX_ANGVEL: f32 = 4.0;

        vel.angvel = if axis.length_squared() > 0.0001 {
            axis.normalize() * (angle * TURN_GAIN).min(MAX_ANGVEL)
        } else {
            Vec3::ZERO
        };
    }
}

fn set_turret_target(
    mut commands: Commands,
    q_focusing: Query<(
        Entity,
        &PilotFocusing,
        &Location,
        Option<&Pilot>,
        Option<&FactionId>,
        Option<&TurretTarget>,
    )>,
    q_targets: Query<(Entity, &Location, Option<&FactionId>, &EntityId), Or<(With<Ship>, With<Station>)>>,
    factions: Res<Factions>,
) {
    for (ship, pilot_focusing, my_loc, pilot, this_faction, tt) in &q_focusing {
        let mut best_target = None;

        if let Some(ent) = pilot_focusing.focusing {
            let can_target = q_targets
                .get(ent)
                .map(|(ent, loc, other_faction, ent_id)| should_be_targetted(&factions, this_faction, *other_faction, ent_id))
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
                .filter(|(ent, loc, other_faction, ent_id)| should_be_targetted(&factions, this_faction, *other_faction, *ent_id))
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
    factions: &Factions,
    this_faction: Option<&FactionId>,
    other_faction: Option<&FactionId>,
    other_ent_id: &EntityId,
) -> bool {
    if let Some(faction) = this_faction.and_then(|f| factions.from_id(f)) {
        let other_fac = other_faction.and_then(|f| factions.from_id(f));
        faction.relation_with_entity(other_ent_id, other_fac) == FactionRelation::Enemy
    } else {
        true
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
            (set_turret_target, look_at_turret_target)
                .chain()
                .in_set(FixedUpdateSet::PrePhysics),
        )
        .register_type::<TurretSystem>();

    register_structure_system::<TurretSystem>(app, false, "cosmos:turret_base");
}
