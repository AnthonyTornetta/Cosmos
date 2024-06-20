use bevy::{
    app::{App, Update},
    prelude::{Commands, Component, Entity, EventReader, IntoSystemConfigs, Query, Res, With, Without},
    reflect::Reflect,
};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{loading::StructureLoadingSet, Structure},
};

fn wire_place_event_listener(
    mut evr_block_updated: EventReader<BlockChangedEvent>,
    registry: Res<Registry<Block>>,
    mut q_wire_graph: Query<&mut WireGraph>,
) {
    let Some(wire_block) = registry.from_id("cosmos:logic_wire") else {
        return;
    };

    let Some(logic_on) = registry.from_id("cosmos:logic_on") else {
        return;
    };

    // ev.block.coords
    // structure.block_info_at(BlockCoordinate)
    // structure.block_rotation(BlockCoordinate).local_front().direction_coordinates()

    for ev in evr_block_updated.read() {
        // If was wire, remove from graph.
        if ev.old_block == wire_block.id() {
            let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) else {
                continue;
            };
        }

        // If is now wire, add to graph.
        if ev.new_block == wire_block.id() {
            let Ok(mut wire_graph) = q_wire_graph.get_mut(ev.structure_entity) else {
                continue;
            };
        }
    }
}

#[derive(Debug, Default, Reflect, Component)]
struct WireGraph {}

fn add_default_wire_graph(q_needs_wire_graph: Query<Entity, (With<Structure>, Without<WireGraph>)>, mut commands: Commands) {
    for entity in q_needs_wire_graph.iter() {
        commands.entity(entity).insert(WireGraph::default());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, add_default_wire_graph.in_set(StructureLoadingSet::AddStructureComponents))
        .register_type::<WireGraph>();
}
