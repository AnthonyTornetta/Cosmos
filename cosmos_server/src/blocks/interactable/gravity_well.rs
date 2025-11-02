use bevy::prelude::*;
use bevy_renet::renet::RenetServer;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockBreakMessage, BlockInteractMessage},
        specific_blocks::gravity_well::GravityWell,
    },
    entities::EntityId,
    netty::{
        NettyChannelServer, cosmos_encoder, server_replication::ReplicationMessage, sync::server_entity_syncing::RequestedEntityMessage,
        system_sets::NetworkingSystemsSet,
    },
    prelude::BlockCoordinate,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::Structure,
    utils::{ecs::FixedUpdateRemovedComponents, ownership::MaybeOwned},
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{EntityIdManager, PersistentComponent, make_persistent};

fn grav_well_handle_block_event(
    mut interact_events: MessageReader<BlockInteractMessage>,
    mut block_break_events: MessageReader<BlockBreakMessage>,
    q_grav_well: Query<&GravityWell>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_has_gravity_wells: Query<Entity, With<GravityWell>>,
    mut commands: Commands,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = q_structure.get(s_block.structure()) else {
            continue;
        };

        if !matches!(structure, Structure::Full(_)) {
            info!("Cannot use gravity well on dynamic structure (like planet) - please send a notification to the player here eventually");
            continue;
        }

        let block = structure.block_at(s_block.coords(), &blocks);

        if block.unlocalized_name() == "cosmos:gravity_well" {
            if let Ok(grav_well) = q_grav_well.get(ev.interactor)
                && grav_well.block == s_block.coords()
                && grav_well.structure_entity == s_block.structure()
            {
                commands.entity(ev.interactor).remove::<GravityWell>();

                continue;
            }

            commands
                .entity(ev.interactor)
                .insert(GravityWell {
                    block: s_block.coords(),
                    g_constant: Vec3::new(0.0, -9.8, 0.0),
                    structure_entity: s_block.structure(),
                })
                .set_parent_in_place(s_block.structure());
        }
    }

    for ev in block_break_events.read() {
        let Ok(structure) = q_structure.get(ev.block.structure()) else {
            continue;
        };

        let block = structure.block_at(ev.block.coords(), &blocks);

        if block.unlocalized_name() != "cosmos:gravity_well" {
            continue;
        }

        for ent in &q_has_gravity_wells {
            commands.entity(ent).remove::<GravityWell>();
        }
    }
}

fn sync_gravity_well(
    mut server: ResMut<RenetServer>,
    q_grav_well: Query<(Entity, &GravityWell), Changed<GravityWell>>,
    removed_components: FixedUpdateRemovedComponents<GravityWell>,
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

fn remove_gravity_wells(mut commands: Commands, q_grav_wells: Query<(Entity, &GravityWell, Option<&ChildOf>)>) {
    for (ent, grav_well, parent) in q_grav_wells.iter() {
        let Some(parent) = parent else {
            commands.entity(ent).remove::<GravityWell>();
            continue;
        };

        if parent.parent() != grav_well.structure_entity {
            commands.entity(ent).remove::<GravityWell>();
        }
    }
}

fn on_request_under_grav(
    mut request_entity_reader: MessageReader<RequestedEntityMessage>,
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

/// The serialized version of a gravity well.
///
/// Only public because the trait requires it to be public. Don't use this.
#[derive(Serialize, Deserialize)]
pub struct SerializedGravityWell {
    /// g_constant * mass = force
    g_constant: Vec3,
    /// The structure this gravity well is for
    structure_entity_id: EntityId,
    /// The block this gravity well is from
    block: BlockCoordinate,
}

impl PersistentComponent for GravityWell {
    type SaveType = SerializedGravityWell;

    fn convert_to_save_type<'a>(&'a self, q_entity_ids: &Query<&EntityId>) -> Option<MaybeOwned<'a, SerializedGravityWell>> {
        q_entity_ids
            .get(self.structure_entity)
            .map(|x| {
                MaybeOwned::Owned(Box::new(SerializedGravityWell {
                    block: self.block,
                    g_constant: self.g_constant,
                    structure_entity_id: *x,
                }))
            })
            .ok()
    }

    fn convert_from_save_type(e_id_type: Self::SaveType, entity_id_manager: &EntityIdManager) -> Option<Self> {
        entity_id_manager
            .entity_from_entity_id(&e_id_type.structure_entity_id)
            .map(|e| Self {
                structure_entity: e,
                g_constant: e_id_type.g_constant,
                block: e_id_type.block,
            })
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<GravityWell>(app);

    app.add_systems(
        FixedUpdate,
        (
            grav_well_handle_block_event,
            remove_gravity_wells,
            sync_gravity_well,
            on_request_under_grav.in_set(NetworkingSystemsSet::SyncComponents),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
