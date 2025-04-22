use bevy::prelude::*;
use cosmos_core::{logic::BlockLogicData, netty::system_sets::NetworkingSystemsSet};

use crate::rendering::structure_renderer::{BlockDataRerenderOnChange, StructureRenderingSet};

fn rerender_on_block_logic_changes(
    mut commands: Commands,
    q_block_logic: Query<Entity, (Without<BlockDataRerenderOnChange>, With<BlockLogicData>)>,
) {
    for ent in q_block_logic.iter() {
        commands.entity(ent).insert(BlockDataRerenderOnChange);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        rerender_on_block_logic_changes
            .before(StructureRenderingSet::MonitorBlockUpdates)
            .in_set(NetworkingSystemsSet::Between),
    );
}
