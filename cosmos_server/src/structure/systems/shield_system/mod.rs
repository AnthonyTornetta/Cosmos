//! Represents all the energy stored on a structure

use bevy::{
    core::Name,
    ecs::{component::Component, entity::Entity, event::Event, query::Changed, schedule::SystemSet},
    hierarchy::BuildChildren,
    log::warn,
    math::Vec3,
    prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, Update},
    transform::{
        components::{GlobalTransform, Transform},
        TransformBundle,
    },
};

use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::Block,
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    netty::{cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, NettyChannelServer},
    persistence::LoadingDistance,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        coordinates::BlockCoordinate,
        events::StructureLoadedEvent,
        loading::StructureLoadingSet,
        shields::Shield,
        systems::{
            shield_system::{ShieldGeneratorBlocks, ShieldGeneratorProperty, ShieldProjectorBlocks, ShieldProjectorProperty, ShieldSystem},
            StructureSystem, StructureSystemType, StructureSystems,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::sync::register_structure_system;

mod explosion;
mod laser;

/*
Shield plan:
Projectors + generators more effective when placed next to other projectors
Shield radius based on max dimensions of projectors
*/

#[derive(Event)]
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
        proj_storage.0.insert(
            block.id(),
            ShieldProjectorProperty {
                shield_range_increase: 1.0,
                shield_strength: 1.0,
            },
        );
    }

    if let Some(block) = blocks.from_id("cosmos:shield_generator") {
        gen_storage.0.insert(
            block.id(),
            ShieldGeneratorProperty {
                efficiency: 0.5,
                max_power_usage_per_sec: 20.0,
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

#[derive(Component)]
struct PlacedShields(Vec<(BlockCoordinate, Entity)>);

fn recalculate_shields_if_needed(
    mut commands: Commands,
    q_placed_shields: Query<&PlacedShields>,
    mut q_shield: Query<&mut Shield>,
    mut q_shield_system: Query<(&mut ShieldSystem, &StructureSystem), Changed<ShieldSystem>>,
    q_structure: Query<(&Location, &GlobalTransform, &Structure)>,
) {
    for (mut shield_system, ss) in &mut q_shield_system {
        if shield_system.needs_shields_recalculated() {
            shield_system.recalculate_shields();
        }

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

                    new_placed_shields.push((coord, ent));
                }
            }

            for &(_, e) in placed_shields.0.iter().filter(|(_, e)| !keep.contains(e)) {
                commands.entity(e).insert(NeedsDespawned);
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
                            strength: shield_details.max_strength,
                        },
                    ))
                    .id();

                new_placed_shields.push((shield_coord, shield_ent));
            }
        });

        commands.entity(structure_entity).insert(PlacedShields(new_placed_shields));
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ShieldHitProcessing {
    OnShieldHit,
}

pub(super) fn register(app: &mut App) {
    laser::register(app);
    explosion::register(app);

    app.configure_sets(Update, ShieldHitProcessing::OnShieldHit);

    app.init_resource::<ShieldProjectorBlocks>()
        .init_resource::<ShieldGeneratorBlocks>()
        .add_systems(OnEnter(GameState::PostLoading), register_energy_blocks)
        .add_systems(
            Update,
            (
                recalculate_shields_if_needed, // before so this runs next frame (so the globaltransform has been added to the structure)
                structure_loaded_event.in_set(StructureLoadingSet::StructureLoaded),
                block_update_system,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, send_shield_hits.after(ShieldHitProcessing::OnShieldHit))
        .register_type::<ShieldSystem>()
        .add_event::<ShieldHitEvent>();

    register_structure_system::<ShieldSystem>(app, false, "cosmos:shield_projector");
}
