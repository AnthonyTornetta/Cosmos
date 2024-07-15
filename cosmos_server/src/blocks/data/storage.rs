use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{EventReader, EventWriter},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
};
use cosmos_core::{
    block::{
        data::BlockData,
        storage::storage_blocks::{on_add_storage, PopulateBlockInventoryEvent},
        Block,
    },
    events::block_events::BlockDataSystemParams,
    inventory::Inventory,
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
    structure::Structure,
};

use crate::persistence::loading::{LoadingBlueprintSystemSet, NeedsBlueprintLoaded, LOADING_SCHEDULE};

fn on_load_blueprint_storage(
    needs_blueprint_loaded_structure: Query<(Entity, &Structure), With<NeedsBlueprintLoaded>>,
    blocks: Res<Registry<Block>>,
    mut ev_writer: EventWriter<PopulateBlockInventoryEvent>,
) {
    for (structure_entity, structure) in needs_blueprint_loaded_structure.iter() {
        let Some(storage_block) = blocks.from_id("cosmos:storage") else {
            return;
        };

        for block in structure.all_blocks_iter(false) {
            if block.block_id(structure) == storage_block.id() {
                ev_writer.send(PopulateBlockInventoryEvent { block, structure_entity });
            }
        }
    }
}

fn populate_inventory(
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    q_has_inventory: Query<(), With<Inventory>>,
    mut params: BlockDataSystemParams,
    mut ev_reader: EventReader<PopulateBlockInventoryEvent>,
) {
    for ev in ev_reader.read() {
        let coords = ev.block.coords();

        let Ok(mut structure) = q_structure.get_mut(ev.structure_entity) else {
            continue;
        };

        structure.insert_block_data_with_entity(
            coords,
            |e| Inventory::new("Storage", 9 * 5, None, e),
            &mut params,
            &mut q_block_data,
            &q_has_inventory,
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        populate_inventory.in_set(NetworkingSystemsSet::Between).after(on_add_storage),
    )
    .add_systems(
        LOADING_SCHEDULE,
        // Need structure to be populated first, thus `DoneLoadingBlueprints` instead of `DoLoadingBlueprints``
        on_load_blueprint_storage.in_set(LoadingBlueprintSystemSet::DoneLoadingBlueprints),
    );
}
