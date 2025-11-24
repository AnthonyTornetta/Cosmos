use bevy::prelude::*;
use cosmos_core::netty::{
    cosmos_encoder,
    sync::{ComponentId, ComponentSyncingSet, GotComponentToRemoveMessage, GotComponentToSyncMessage, mapping::NetworkMapping},
};

fn client_deserialize_parent(
    mut ev_reader: MessageReader<GotComponentToSyncMessage>,
    mut commands: Commands,
    mapping: Res<NetworkMapping>,
) {
    for ev in ev_reader.read() {
        if !matches!(ev.component_id, ComponentId::ChildOf) {
            continue;
        }

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
            let new_parent = cosmos_encoder::deserialize_uncompressed::<Entity>(&ev.raw_data)
                .expect("Failed to deserialize component sent from server!");

            let Some(mapped_parent) = mapping.client_from_server(&new_parent) else {
                warn!("Couldn't convert entities for parent {new_parent:?}!");
                continue;
            };

            ecmds.set_parent_in_place(mapped_parent);
        } else {
            warn!("No entity cmds for synced entity component - (entity {:?})", ev.entity);
        }
    }
}

fn client_remove_parent(mut ev_reader: MessageReader<GotComponentToRemoveMessage>, mut commands: Commands) {
    for ev in ev_reader.read() {
        if !matches!(ev.component_id, ComponentId::ChildOf) {
            continue;
        }

        if let Ok(mut ecmds) = commands.get_entity(ev.entity) {
            ecmds.remove_parent_in_place();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (client_deserialize_parent, client_remove_parent)
            .chain()
            .run_if(resource_exists::<NetworkMapping>)
            .in_set(ComponentSyncingSet::ReceiveComponents),
    );
}
