use bevy::prelude::*;
use cosmos_core::{block::specific_blocks::numeric_display::NumericDisplayValue, netty::system_sets::NetworkingSystemsSet};

use crate::rendering::structure_renderer::BlockDataRerenderOnChange;

fn add_render_flag(q_entity: Query<Entity, Added<NumericDisplayValue>>, mut commands: Commands) {
    for e in q_entity.iter() {
        commands.entity(e).insert(BlockDataRerenderOnChange);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, add_render_flag.in_set(NetworkingSystemsSet::Between));
}
