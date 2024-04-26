//! Provides default syncing support for bevy components

use bevy::{app::App, hierarchy::Parent};

use super::{sync_component, SyncableComponent};

// impl SyncableComponent for Parent {
//     fn get_component_unlocalized_name() -> &'static str {
//         "bevy:parent"
//     }

//     fn get_sync_type() -> super::SyncType {
//         super::SyncType::ServerAuthoritative
//     }

//     #[cfg(feature = "client")]
//     fn convert_entities_server_to_client(self, _mapping: &super::mapping::NetworkMapping) -> Option<Self> {
//         _mapping.client_from_server(&self.get()).map(|e| Parent(e))
//     }
// }

pub(super) fn register(app: &mut App) {
    // sync_component::<Parent>(app);
}
