//! Represents all the energy stored on a structure

use std::time::Duration;

use bevy::{
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        query::{Added, Changed, Or, With},
        schedule::SystemSet,
    },
    hierarchy::{BuildChildren, Parent},
    log::warn,
    math::Vec3,
    prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, IntoSystemSetConfigs, OnEnter, Query, Res, ResMut, Update},
    reflect::Reflect,
    time::Time,
    transform::{
        bundles::TransformBundle,
        components::{GlobalTransform, Transform},
    },
    utils::hashbrown::HashMap,
};

use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::location::Location,
    projectiles::laser::LaserSystemSet,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::BlockCoordinate,
        events::StructureLoadedEvent,
        shields::Shield,
        systems::{
            energy_storage_system::EnergyStorageSystem,
            shield_system::{ShieldGeneratorBlocks, ShieldGeneratorProperty, ShieldProjectorBlocks, ShieldProjectorProperty, ShieldSystem},
            StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet,
        },
        Structure,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    ai::AiControlled,
    persistence::{
        loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    state::GameState,
};

use super::sync::register_structure_system;

mod explosion;
mod laser;

/*
Shield plan:
Projectors + generators more effective when placed next to other projectors
Shield radius based on max dimensions of projectors
*/

#[derive(Event)]
/// Sent when a shield is hit
pub struct ShieldHitEvent {
    shield_entity: Entity,
    relative_position: Vec3,
}

fn register_energy_blocks(
    blocks: Res<Registry<Block>>,
    mut gen_storage: ResMut<ShieldGeneratorBlocks>,
    mut proj_storage: ResMut<ShieldProjectorBlocks>,
) {
    if let Some(block) = blocks.from_id("cosmos:shield_projector") {
        proj_storage.0.insert(block.id(), ShieldProjectorProperty { shield_strength: 1.0 });
    }

    if let Some(block) = blocks.from_id("cosmos:shield_generator") {
        gen_storage.0.insert(
            block.id(),
            ShieldGeneratorProperty {
                peak_efficiency: 0.85,
                power_usage_per_sec: 20.0,
            },
        );
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    shield_projector_blocks: Res<ShieldProjectorBlocks>,
    shield_generator_blocks: Res<ShieldGeneratorBlocks>,
    mut system_query: Query<&mut ShieldSystem>,
    systems_query: Query<&StructureSystems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if shield_projector_blocks.0.get(&ev.old_block).is_some() {
                    system.projector_removed(ev.block.coords());
                }

                if let Some(&prop) = shield_projector_blocks.0.get(&ev.new_block) {
                    system.projector_added(prop, ev.block.coords());
                }

                if shield_generator_blocks.0.get(&ev.old_block).is_some() {
                    system.generator_removed(ev.block.coords());
                }

                if let Some(&prop) = shield_generator_blocks.0.get(&ev.new_block) {
                    system.generator_added(prop, ev.block.coords());
                }
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    mut commands: Commands,
    shield_projector_blocks: Res<ShieldProjectorBlocks>,
    shield_generator_blocks: Res<ShieldGeneratorBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = ShieldSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(&prop) = shield_projector_blocks.0.get(&structure.block_id_at(block.coords())) {
                    system.projector_added(prop, block.coords());
                }

                if let Some(&prop) = shield_generator_blocks.0.get(&structure.block_id_at(block.coords())) {
                    system.generator_added(prop, block.coords());
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

fn send_shield_hits(mut ev_reader: EventReader<ShieldHitEvent>, mut server: ResMut<RenetServer>) {
    for ev in ev_reader.read() {
        server.broadcast_message(
            NettyChannelServer::StructureSystems,
            cosmos_encoder::serialize(&ServerStructureSystemMessages::ShieldHit {
                shield_entity: ev.shield_entity,
                relative_location: ev.relative_position,
            }),
        );
    }
}

#[derive(Debug, Component, Clone, Default, Reflect)]
struct PlacedShields(Vec<(BlockCoordinate, Entity)>);

fn recalculate_shields_if_needed(
    mut commands: Commands,
    q_placed_shields: Query<&PlacedShields>,
    mut q_shield: Query<&mut Shield>,
    mut q_shield_system: Query<(&mut ShieldSystem, &StructureSystem), Changed<ShieldSystem>>,
    q_structure: Query<(&Location, &GlobalTransform, &Structure)>,
) {
    for (mut shield_system, ss) in &mut q_shield_system {
        if !shield_system.needs_shields_recalculated() {
            continue;
        }

        shield_system.recalculate_shields();

        let mut keep = vec![];
        let mut done = vec![];

        let shield_details = shield_system.shield_details();

        let structure_entity = ss.structure_entity();

        let mut new_placed_shields = vec![];

        if let Ok(placed_shields) = q_placed_shields.get(structure_entity) {
            for &(cur_shield_coord, shield_details) in shield_details {
                if let Some(&(coord, ent)) = placed_shields.0.iter().find(|(bc, _)| *bc == cur_shield_coord) {
                    keep.push(ent);
                    done.push(coord);

                    let Ok(mut shield) = q_shield.get_mut(ent) else {
                        warn!("This shouldn't be possible.");
                        continue;
                    };

                    shield.max_strength = shield_details.max_strength;
                    shield.radius = shield_details.radius;
                    shield.strength = shield.strength.min(shield_details.max_strength);
                    shield.power_per_second = shield_details.generation_power_per_sec;
                    shield.power_efficiency = shield_details.generation_efficiency;

                    new_placed_shields.push((coord, ent));
                }
            }

            for &(_, e) in placed_shields.0.iter().filter(|(_, e)| !keep.contains(e)) {
                if let Some(mut ecmds) = commands.get_entity(e) {
                    ecmds.insert(NeedsDespawned);
                }
            }
        }

        let Ok((loc, g_trans, structure)) = q_structure.get(structure_entity) else {
            warn!("No loc/g-trans/structure");
            continue;
        };

        commands.entity(structure_entity).with_children(|p| {
            for &(shield_coord, shield_details) in shield_details.iter().filter(|(c, _)| !done.contains(c)) {
                let shield_pos = structure.block_relative_position(shield_coord);
                // Locations don't account for parent rotation
                let shield_loc = g_trans.affine().matrix3.mul_vec3(shield_pos);

                let shield_ent = p
                    .spawn((
                        Name::new("Shield"),
                        TransformBundle::from_transform(Transform::from_translation(shield_pos)),
                        *loc + shield_loc,
                        LoadingDistance::new(1, 2),
                        Shield {
                            max_strength: shield_details.max_strength,
                            radius: shield_details.radius,
                            block_coord: shield_coord,
                            strength: 0.0,
                            power_efficiency: shield_details.generation_efficiency,
                            power_per_second: shield_details.generation_power_per_sec,
                        },
                    ))
                    .id();

                new_placed_shields.push((shield_coord, shield_ent));
            }
        });

        commands.entity(structure_entity).insert(PlacedShields(new_placed_shields));
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Reflect)]
struct ShieldDowntime(f32);

const MAX_SHIELD_DOWNTIME: Duration = Duration::from_secs(10);

fn power_shields(
    mut commands: Commands,
    mut q_storage_system: Query<&mut EnergyStorageSystem>,
    q_systems: Query<&StructureSystems>,
    mut q_shields: Query<(Entity, &mut Shield, &Parent, Option<&mut ShieldDowntime>)>,
    time: Res<Time>,
) {
    for (ent, mut shield, parent, shield_downtime) in &mut q_shields {
        if shield.strength < shield.max_strength {
            if shield.strength == 0.0 {
                let Some(mut shield_downtime) = shield_downtime else {
                    commands.entity(ent).insert(ShieldDowntime(time.delta_seconds()));
                    continue;
                };

                if shield_downtime.0 < MAX_SHIELD_DOWNTIME.as_secs_f32() {
                    shield_downtime.0 += time.delta_seconds();
                    continue;
                }
            }

            let strength_missing = shield.max_strength - shield.strength;

            let optimal_power_usage = strength_missing / shield.power_efficiency;
            let power_usage = optimal_power_usage.min(shield.power_per_second * time.delta_seconds());

            let Ok(systems) = q_systems.get(parent.get()) else {
                warn!("Shield's parent isn't a structure?");
                continue;
            };

            let Ok(mut ecs) = systems.query_mut(&mut q_storage_system) else {
                warn!("Structure w/ shield missing energy storage system!");
                continue;
            };

            let not_used = ecs.decrease_energy(power_usage);

            let old_strength = shield.strength;
            shield.strength += (power_usage - not_used) * shield.power_efficiency;

            if old_strength == 0.0 && shield.strength != 0.0 {
                commands.entity(ent).remove::<ShieldDowntime>();
            }
        }
    }
}

/// AIs should spawn with full shield systems
fn fill_ai_controlled_shields_on_spawn(
    q_structure: Query<&PlacedShields, (With<AiControlled>, Or<(Added<AiControlled>, Added<PlacedShields>)>)>,
    mut q_shield: Query<&mut Shield>,
) {
    for placed_shields in q_structure.iter() {
        for &(_, shield_ent) in placed_shields.0.iter() {
            let Ok(mut shield) = q_shield.get_mut(shield_ent) else {
                continue;
            };

            if shield.power_per_second > 0.0 && shield.power_efficiency > 0.0 {
                shield.strength = shield.max_strength;
            }
        }
    }
}

fn on_save_shield(mut q_needs_saved: Query<(&Shield, Option<&ShieldDowntime>, &mut SerializedData), With<NeedsSaved>>) {
    for (shield, downtime, mut sd) in q_needs_saved.iter_mut() {
        sd.serialize_data("cosmos:shield", shield);
        if let Some(dt) = downtime {
            sd.serialize_data("cosmos:shield_downtime", dt);
        }
    }
}

fn on_load_shield(
    mut commands: Commands,
    q_placed_shields: Query<&PlacedShields>,
    q_needs_saved: Query<(Entity, &SerializedData, &Parent), With<NeedsLoaded>>,
) {
    let mut hm = HashMap::new();

    for (ent, sd, parent) in q_needs_saved.iter() {
        if let Some(shield) = sd.deserialize_data::<Shield>("cosmos:shield") {
            hm.entry(parent.get()).or_insert(Vec::new()).push((ent, shield.block_coord));
            commands.entity(ent).insert((shield, Name::new("Shield")));

            if let Some(downtime) = sd.deserialize_data::<ShieldDowntime>("cosmos:shield_downtime") {
                commands.entity(ent).insert(downtime);
            }
        }
    }

    for (parent, shields) in hm {
        // The parent shouldn't ever have any placed shields since it should also be loading in, but juuuuust in case we grab it.
        let mut placed_shields = q_placed_shields.get(parent).cloned().unwrap_or_default();

        for (ent, bc) in shields {
            placed_shields.0.push((bc, ent));
        }

        commands.entity(parent).insert(placed_shields);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Shield logic
pub enum ShieldSet {
    /// Shields consume power and recharge themselves
    RechargeShields,
    /// Shields receive incoming damage and deplete their charge (potentially going down)
    OnShieldHit,
}

pub(super) fn register(app: &mut App) {
    laser::register(app);
    explosion::register(app);

    app.configure_sets(
        Update,
        (
            ShieldSet::RechargeShields,
            ShieldSet::OnShieldHit.after(LaserSystemSet::SendHitEvents),
        )
            .chain(),
    );

    app.init_resource::<ShieldProjectorBlocks>()
        .init_resource::<ShieldGeneratorBlocks>()
        .add_systems(OnEnter(GameState::PostLoading), register_energy_blocks)
        .add_systems(
            Update,
            (
                recalculate_shields_if_needed, // before so this runs next frame (so the globaltransform has been added to the structure)
                fill_ai_controlled_shields_on_spawn,
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
                power_shields,
            )
                .chain()
                .in_set(ShieldSet::RechargeShields)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            send_shield_hits
                .in_set(NetworkingSystemsSet::SyncComponents)
                .after(ShieldSet::OnShieldHit),
        )
        .add_systems(SAVING_SCHEDULE, on_save_shield.in_set(SavingSystemSet::DoSaving))
        .add_systems(LOADING_SCHEDULE, on_load_shield.in_set(LoadingSystemSet::DoLoading))
        .register_type::<ShieldSystem>()
        .add_event::<ShieldHitEvent>()
        .register_type::<ShieldDowntime>()
        .register_type::<PlacedShields>();

    register_structure_system::<ShieldSystem>(app, false, "cosmos:shield_projector");
}
