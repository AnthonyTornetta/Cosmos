use crate::state::GameState;
use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::EventReader,
        query::Changed,
        removal_detection::RemovedComponents,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Commands, Query, ResMut},
    },
    hierarchy::{BuildChildren, Parent},
    log::info,
    math::Vec3,
    prelude::Res,
};
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        block_events::{BlockBreakEvent, BlockInteractEvent},
        specific_blocks::gravity_well::GravityWell,
        Block,
    },
    netty::{
        cosmos_encoder, server_replication::ReplicationMessage, sync::server_entity_syncing::RequestedEntityEvent, NettyChannelServer,
    },
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

fn grav_well_handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    mut block_break_events: EventReader<BlockBreakEvent>,
    q_grav_well: Query<&GravityWell>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_has_gravity_wells: Query<(Entity, &GravityWell)>,
    mut commands: Commands,
) {
    for ev in interact_events.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        if !matches!(structure, Structure::Full(_)) {
            info!("Cannot use gravity well on dynamic structure (like planet) - please send a notification to the player here eventually");
            continue;
        }

        let block = structure.block_at(ev.structure_block.coords(), &blocks);

        if block.unlocalized_name() == "cosmos:gravity_well" {
            if let Ok(grav_well) = q_grav_well.get(ev.interactor) {
                if grav_well.block == ev.structure_block.coords() && grav_well.structure_entity == ev.structure_entity {
                    commands.entity(ev.interactor).remove::<GravityWell>();

                    continue;
                }
            }

            commands
                .entity(ev.interactor)
                .insert(GravityWell {
                    block: ev.structure_block.coords(),
                    g_constant: Vec3::new(0.0, -9.8, 0.0),
                    structure_entity: ev.structure_entity,
                })
                .set_parent(ev.structure_entity);
        }
    }

    for ev in block_break_events.read() {
        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = structure.block_at(ev.block.coords(), &blocks);

        if block.unlocalized_name() != "cosmos:gravity_well" {
            continue;
        }

        for (ent, grav_well) in &q_has_gravity_wells {
            if grav_well.block == ev.block.coords() && grav_well.structure_entity == ev.structure_entity {
                commands.entity(ent).remove::<GravityWell>();
            }
        }
    }
}

fn sync_gravity_well(
    mut server: ResMut<RenetServer>,
    q_grav_well: Query<(Entity, &GravityWell), Changed<GravityWell>>,
    mut removed_components: RemovedComponents<GravityWell>,
) {
    for (entity, under_grav_well) in &q_grav_well {
        server.broadcast_message(
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::GravityWell {
                gravity_well: Some(*under_grav_well),
                entity,
            }),
        );
    }

    for entity in removed_components.read() {
        server.broadcast_message(
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::GravityWell {
                gravity_well: None,
                entity,
            }),
        );
    }
}

fn remove_gravity_wells(mut commands: Commands, q_grav_wells: Query<(Entity, &GravityWell, Option<&Parent>)>) {
    for (ent, grav_well, parent) in q_grav_wells.iter() {
        let Some(parent) = parent else {
            commands.entity(ent).remove::<GravityWell>();
            continue;
        };

        if parent.get() != grav_well.structure_entity {
            commands.entity(ent).remove::<GravityWell>();
        }
    }
}

fn on_request_under_grav(
    mut request_entity_reader: EventReader<RequestedEntityEvent>,
    mut server: ResMut<RenetServer>,
    q_grav_well: Query<&GravityWell>,
) {
    for ev in request_entity_reader.read() {
        let Ok(grav_well) = q_grav_well.get(ev.entity) else {
            continue;
        };

        server.send_message(
            ev.client_id,
            NettyChannelServer::SystemReplication,
            cosmos_encoder::serialize(&ReplicationMessage::GravityWell {
                gravity_well: Some(*grav_well),
                entity: ev.entity,
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            grav_well_handle_block_event,
            remove_gravity_wells,
            sync_gravity_well,
            on_request_under_grav,
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
