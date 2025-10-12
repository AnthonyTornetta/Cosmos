use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rapier3d::prelude::ReadMassProperties;
use cosmos_core::{
    block::{Block, block_events::BlockEventsSet},
    ecs::commands::OwnedOrMut,
    events::{block_events::BlockChangedEvent, structure::structure_event::StructureEventIterator},
    physics::location::{Location, SECTOR_DIMENSIONS},
    prelude::{Structure, StructureLoadedEvent, StructureSystem, StructureSystems},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::systems::{
        StructureSystemOrdering, StructureSystemType, StructureSystemsSet, SystemActive,
        energy_storage_system::EnergyStorageSystem,
        warp::warp_drive::{WarpBlockProperty, WarpDriveSystem},
    },
};

use crate::{
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    structure::systems::sync::register_structure_system,
};

#[derive(Resource, Debug, Default)]
struct WarpDriveBlocks(HashMap<u16, WarpBlockProperty>);

impl WarpDriveBlocks {
    pub fn get(&self, block_id: u16) -> Option<WarpBlockProperty> {
        self.0.get(&block_id).copied()
    }

    pub fn insert(&mut self, block: &Block, property: WarpBlockProperty) {
        self.0.insert(block.id(), property);
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    warp_blocks: Res<WarpDriveBlocks>,
    mut q_system: Query<(&StructureSystem, &mut WarpDriveSystem)>,
    mut q_systems: Query<(&mut StructureSystems, &mut StructureSystemOrdering)>,
    mut commands: Commands,
    systems_registry: Res<Registry<StructureSystemType>>,
) {
    for (structure, ev) in event.read().group_by_structure() {
        let Ok((mut systems, mut ordering)) = q_systems.get_mut(structure) else {
            continue;
        };

        let (structure_system, mut system) = systems
            .query_mut(&mut q_system)
            .map(|(ss, x)| (Some(ss), OwnedOrMut::Mut(x)))
            .unwrap_or_else(|_| (None, OwnedOrMut::Owned(Default::default())));

        for ev in ev {
            if let Some(prop) = warp_blocks.get(ev.old_block) {
                system.remove_warp_block(ev.block.coords(), prop);
            }

            if let Some(prop) = warp_blocks.get(ev.new_block) {
                system.add_warp_block(ev.block.coords(), prop);
            }
        }

        if system.empty()
            && let Some(structure_system) = structure_system.copied()
        {
            systems.remove_system(&mut commands, &structure_system, &systems_registry, ordering.as_mut());
        } else if let Some(system) = system.owned() {
            systems.add_system(&mut commands, system, &systems_registry);
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems, &mut StructureSystemOrdering)>,
    warp_blocks: Res<WarpDriveBlocks>,
    mut q_system: Query<(&StructureSystem, &mut WarpDriveSystem)>,
    mut commands: Commands,
    systems_registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems, mut ordering)) = structure_query.get_mut(ev.structure_entity) {
            let (structure_system, mut system) = systems
                .query_mut(&mut q_system)
                .map(|(ss, x)| (Some(ss), OwnedOrMut::Mut(x)))
                .unwrap_or_else(|_| (None, OwnedOrMut::Owned(Default::default())));

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = warp_blocks.get(structure.block_id_at(block)) {
                    system.add_warp_block(block, prop);
                }
            }

            if system.empty()
                && let Some(structure_system) = structure_system.copied()
            {
                systems.remove_system(&mut commands, &structure_system, &systems_registry, ordering.as_mut());
            } else if let Some(system) = system.owned() {
                systems.add_system(&mut commands, system, &systems_registry);
            }
        }
    }
}

fn register_warp_blocks(mut warp_blocks: ResMut<WarpDriveBlocks>, blocks: Res<Registry<Block>>) {
    if let Some(b) = blocks.from_id("cosmos:warp_drive") {
        warp_blocks.insert(
            b,
            WarpBlockProperty {
                charge_per_tick: 100,
                capacitance: 10_000,
            },
        )
    }
}

fn on_activate_system(
    mut q_active: Query<(&mut WarpDriveSystem, &StructureSystem), With<SystemActive>>,
    mut q_systems: Query<(&mut Location, &Transform, &ReadMassProperties), Without<ChildOf>>,
) {
    const MAX_JUMP_DIST: f32 = SECTOR_DIMENSIONS * 50.0;

    for (mut warp, ss) in q_active.iter_mut() {
        let Ok((mut loc, trans, mass)) = q_systems.get_mut(ss.structure_entity()) else {
            continue;
        };

        if !warp.can_jump(mass.get().mass) {
            continue;
        }

        warp.discharge();

        *loc = *loc + Location::new((trans.rotation * Vec3::NEG_Z) * MAX_JUMP_DIST, Default::default());
    }
}

fn charge_warp_drive(
    mut q_warp: Query<(&mut WarpDriveSystem, &StructureSystem)>,
    q_systems: Query<(&StructureSystems, &ReadMassProperties)>,
    mut q_ess: Query<&mut EnergyStorageSystem>,
) {
    for (mut warp, ss) in q_warp.iter_mut() {
        let Ok((systems, mass)) = q_systems.get(ss.structure_entity()) else {
            continue;
        };

        if warp.can_jump(mass.get().mass) {
            // Don't keep charging if we can jump
            continue;
        }

        let Ok(mut ess) = systems.query_mut(&mut q_ess) else {
            continue;
        };

        let mut charge_amt = warp.charge_per_tick();

        let leftover = (ess.decrease_energy((charge_amt * 10) as f32) / 10.0).ceil();
        charge_amt -= leftover as u32;

        warp.increase_charge(charge_amt);
    }
}

impl DefaultPersistentComponent for WarpDriveSystem {}

pub(super) fn register(app: &mut App) {
    make_persistent::<WarpDriveSystem>(app);

    app.init_resource::<WarpDriveBlocks>()
        .add_systems(OnEnter(GameState::PostLoading), register_warp_blocks)
        .add_systems(
            FixedUpdate,
            (charge_warp_drive, on_activate_system)
                .chain()
                .in_set(StructureSystemsSet::UpdateSystems),
        )
        .add_systems(
            FixedUpdate,
            (
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .run_if(in_state(GameState::Playing)),
        );

    register_structure_system::<WarpDriveSystem>(app, true, "cosmos:warp_drive");
}
