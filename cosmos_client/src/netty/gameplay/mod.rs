use bevy::prelude::{resource_exists, App, IntoSystemConfigs, RemovedComponents, ResMut, Update};
use cosmos_core::physics::location::Location;

use super::mapping::NetworkMapping;

mod receiver;
mod sync;

/// This assumes that when an entity is removed, its location component will also be removed.
///
/// Thus, removing any location from anything will make the game think it was despawned
///
/// Please come up with a better solution
fn remove_despawned_entities(mut removed_entities: RemovedComponents<Location>, mut mapping: ResMut<NetworkMapping>) {
    for removed in removed_entities.read() {
        mapping.remove_mapping_from_client_entity(&removed);
    }
}

pub(super) fn register(app: &mut App) {
    sync::register(app);
    receiver::register(app);

    app.add_systems(Update, remove_despawned_entities.run_if(resource_exists::<NetworkMapping>()));
}
