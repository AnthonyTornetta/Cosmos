//! Make blocks have default data easily.

use std::marker::PhantomData;

use bevy::{prelude::*, utils::HashMap};
use cosmos_core::{
    block::{block_events::BlockEventsSet, data::BlockData, Block},
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    netty::system_sets::NetworkingSystemsSet,
    prelude::{BlockCoordinate, Structure, StructureLoadingSet},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

use crate::{
    fluid::interact_fluid::FluidInteractionSet,
    persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
};

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
    initializers: Res<BlockDataInitializers<T>>,
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut evw_bd: EventWriter<InsertBlockDataEvent<T>>,
) {
    if evr_block_changed.is_empty() {
        return Default::default();
    }

    let mut blocks: HashMap<Entity, Vec<BlockCoordinate>> = HashMap::default();

    for ev in evr_block_changed.read() {
        if ev.new_block == ev.old_block {
            continue;
        }

        if !initializers.block_ids.contains_key(&ev.new_block) {
            continue;
        }

        blocks.entry(ev.block.structure()).or_default().push(ev.block.coords());
    }

    evw_bd.send(InsertBlockDataEvent(blocks, Default::default()));
}

#[derive(Event)]
struct InsertBlockDataEvent<T: Component>(HashMap<Entity, Vec<BlockCoordinate>>, PhantomData<T>);

fn insert_block_data<T: Component>(
    mut evr_todo: EventReader<InsertBlockDataEvent<T>>,
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut params: BlockDataSystemParams,
    q_has_component: Query<(), With<T>>,
    initializers: Res<BlockDataInitializers<T>>,
    mut commands: Commands,
) {
    for InsertBlockDataEvent(input, _) in evr_todo.read() {
        for (&entity, coords) in input.iter() {
            let Ok(mut structure) = q_structure.get_mut(entity) else {
                continue;
            };

            for &c in coords {
                let Some(initializer) = initializers.block_ids.get(&structure.block_id_at(c)) else {
                    continue;
                };

                if structure.query_block_data(c, &q_has_component).is_some() {
                    continue;
                }

                if structure
                    .insert_block_data_with_entity(
                        c,
                        |e| initializer(e, &mut commands),
                        &mut params,
                        &mut q_block_data,
                        &q_has_component,
                    )
                    .is_none()
                {
                    warn!("Error inserting default block data - chunk not loaded!!");
                }
            }
        }
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
        app.add_event::<InsertBlockDataEvent<T>>();

        app.add_systems(
            Update,
            (
                on_load_blueprint_storage::<T>.in_set(LoadingBlueprintSystemSet::DoneLoadingBlueprints),
                (
                    insert_block_data::<T>.in_set(StructureLoadingSet::InitializeChunkBlockData),
                    on_add_block::<T>
                        .in_set(BlockEventsSet::ProcessEvents)
                        .ambiguous_with(FluidInteractionSet::InteractWithFluidBlocks),
                )
                    .chain()
                    .in_set(NetworkingSystemsSet::Between),
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
