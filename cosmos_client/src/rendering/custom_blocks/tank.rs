use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        schedule::{IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    hierarchy::BuildChildren,
    reflect::Reflect,
};
use cosmos_core::{
    block::Block,
    ecs::NeedsDespawned,
    fluid::data::StoredBlockFluid,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

use crate::{
    rendering::structure_renderer::{BlockRenderingModes, ChunkNeedsCustomBlocksRendered, RenderingMode, StructureRenderingSet},
    state::game_state::GameState,
};

fn set_custom_rendering_for_tank(mut rendering_modes: ResMut<BlockRenderingModes>, blocks: Res<Registry<Block>>) {
    if let Some(tank) = blocks.from_id("cosmos:tank") {
        rendering_modes.set_rendering_mode(tank, RenderingMode::Both);
    }
}

#[derive(Component, Reflect)]
struct TankRenders(Vec<Entity>);

fn on_render_tanks(
    q_tank_renders: Query<&TankRenders>,
    mut ev_reader: EventReader<ChunkNeedsCustomBlocksRendered>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    q_structure: Query<&Structure>,
    q_stored_fluid: Query<&StoredBlockFluid>,
) {
    for ev in ev_reader.read() {
        if let Ok(tank_renders) = q_tank_renders.get(ev.mesh_entity_parent) {
            for e in tank_renders.0.iter().copied() {
                commands.entity(e).insert(NeedsDespawned);
            }

            commands.entity(ev.mesh_entity_parent).remove::<TankRenders>();
        }

        let tank_id = blocks.from_id("cosmos:tank").expect("no tank :(").id();
        if ev.block_ids.contains(&tank_id) {
            println!("Custom render contains tank!");

            let ent = commands.spawn(Name::new("Fake Render!")).set_parent(ev.mesh_entity_parent).id();

            let Ok(structure) = q_structure.get(ev.structure_entity) else {
                continue;
            };

            for block in structure.block_iter_for_chunk(ev.chunk_coordinate, true) {
                if structure.block_id_at(block.coords()) != tank_id {
                    continue;
                }

                let Some(data) = structure.query_block_data(block.coords(), &q_stored_fluid) else {
                    continue;
                };

                println!("Do special rendering for: {data:?}");
            }

            commands.entity(ev.mesh_entity_parent).insert(TankRenders(vec![ent]));
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), set_custom_rendering_for_tank);

    app.add_systems(Update, on_render_tanks.in_set(StructureRenderingSet::CustomRendering));

    app.register_type::<TankRenders>();
}
