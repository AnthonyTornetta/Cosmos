//! Make blocks have default data easily.

use bevy::{prelude::*, utils::HashMap};
use cosmos_core::{
    block::{block_events::BlockEventsSet, data::BlockData, Block},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    prelude::{BlockCoordinate, Structure},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

use crate::{fluid::interact_fluid::FluidInteractionSet, persistence::loading::NeedsBlueprintLoaded};

type DataConstructor<T> = fn(Entity, &mut Commands) -> T;

#[derive(Resource)]
struct BlockDataInitializers<T: Component> {
    block_ids: HashMap<u16, DataConstructor<T>>,
}

impl<T: Component> Default for BlockDataInitializers<T> {
    fn default() -> Self {
        Self {
            block_ids: Default::default(),
        }
    }
}

/// Used to process the addition/removal of storage blocks to a structure.
///
/// Sends out the `PopulateBlockInventoryEvent` event when needed.
fn on_add_block<T: Component>(
    mut q_structure: Query<&mut Structure>,
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut q_block_data: Query<&mut BlockData>,
    mut params: BlockDataSystemParams,
    q_has_component: Query<(), With<T>>,
    initializers: Res<BlockDataInitializers<T>>,
    mut commands: Commands,
) {
    if evr_block_changed.is_empty() {
        return;
    }

    for ev in evr_block_changed.read() {
        if ev.new_block == ev.old_block {
            continue;
        }

        let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };

        let Some(initializer) = initializers.block_ids.get(&ev.new_block) else {
            continue;
        };

        if structure.query_block_data(ev.block.coords(), &q_has_component).is_some() {
            continue;
        }

        structure.insert_block_data_with_entity(
            ev.block.coords(),
            |e| initializer(e, &mut commands),
            &mut params,
            &mut q_block_data,
            &q_has_component,
        );
    }
}

fn on_load_blueprint_storage<T: Component>(
    mut needs_blueprint_loaded_structure: Query<&mut Structure, With<NeedsBlueprintLoaded>>,
    initializers: Res<BlockDataInitializers<T>>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_component: Query<(), With<T>>,
    mut params: BlockDataSystemParams,
    mut commands: Commands,
) {
    for structure in needs_blueprint_loaded_structure.iter_mut() {
        init_block_data(
            &initializers,
            &mut q_block_data,
            &q_has_component,
            &mut params,
            &mut commands,
            structure,
        );
    }
}

fn init_block_data<T: Component>(
    initializers: &BlockDataInitializers<T>,
    q_block_data: &mut Query<&mut BlockData>,
    q_has_component: &Query<(), With<T>>,
    params: &mut BlockDataSystemParams,
    commands: &mut Commands,
    mut structure: Mut<Structure>,
) {
    for (block, initializer) in structure
        .all_blocks_iter(false)
        .flat_map(|bc| {
            initializers
                .block_ids
                .get(&structure.block_id_at(bc))
                .map(|initializer| (bc, initializer))
        })
        .filter(|(bc, _)| structure.query_block_data(*bc, q_has_component).is_none())
        .collect::<Vec<(BlockCoordinate, &DataConstructor<T>)>>()
    {
        structure.insert_block_data_with_entity(block, |e| initializer(e, commands), params, q_block_data, q_has_component);
    }
}

pub fn add_default_block_data_for_block<T: Component>(app: &mut App, ctor: DataConstructor<T>, block_id: &'static str) {
    if !app.world().contains_resource::<BlockDataInitializers<T>>() {
        app.init_resource::<BlockDataInitializers<T>>();
        app.add_systems(
            Update,
            (
                on_load_blueprint_storage::<T>
                    .in_set(BlockEventsSet::ProcessEvents)
                    .ambiguous_with(FluidInteractionSet::InteractWithFluidBlocks),
                on_add_block::<T>.in_set(BlockEventsSet::SendEventsForNextFrame),
            ),
        );
    }

    app.add_systems(
        OnExit(GameState::PostLoading),
        move |blocks: Res<Registry<Block>>, mut bd_inits: ResMut<BlockDataInitializers<T>>| {
            let Some(block) = blocks.from_id(block_id) else {
                error!("Block id {block_id} not found! Cannot add default block data for it!");
                return;
            };

            bd_inits.block_ids.insert(block.id(), ctor);
        },
    );
}
