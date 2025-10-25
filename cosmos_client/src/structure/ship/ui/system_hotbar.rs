//! Displays the ships's system selection hotbar

use bevy::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    inventory::{
        Inventory,
        itemstack::{ItemShouldHaveData, ItemStackSystemSet},
    },
    item::{Item, usable::cooldown::ItemCooldown},
    netty::client::LocalPlayer,
    registry::Registry,
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::{StructureSystem, StructureSystemCharge, StructureSystemOrdering, StructureSystemType, StructureSystems},
    },
};

use crate::{
    structure::systems::player_interactions::{HoveredSystem, SystemUsageSet},
    ui::hotbar::{Hotbar, HotbarContents, HotbarPriorityQueue, LocalPlayerHotbar},
};

const SHIP_PRIORITY_IDENTIFIER: &str = "cosmos:ship_systems";

#[derive(Component)]
struct PilotStructureSystemsInventory;

fn create_pilot_inventory(
    q_pilot: Query<(), (With<LocalPlayer>, With<Pilot>)>,
    q_exists_already: Query<(), (With<PilotStructureSystemsInventory>, Without<NeedsDespawned>)>,
    mut commands: Commands,
) {
    if !(!q_pilot.is_empty() && q_exists_already.is_empty()) {
        return;
    }

    let mut ecmds = commands.spawn((PilotStructureSystemsInventory,));

    let inv = Inventory::new("", 9, None, ecmds.id());
    ecmds.insert(inv);
}

fn remove_pilot_inventory(
    q_pilot: Query<(), (With<LocalPlayer>, With<Pilot>)>,
    q_exists_already: Query<Entity, With<PilotStructureSystemsInventory>>,
    mut commands: Commands,
) {
    if !q_pilot.is_empty() {
        return;
    }

    for e in q_exists_already.iter() {
        commands.entity(e).insert(NeedsDespawned);
    }
}

fn add_priority_when_flying(
    mut q_hotbar_priority: Query<&mut HotbarPriorityQueue, With<LocalPlayerHotbar>>,
    q_started_flying: Query<(), (Added<Pilot>, With<LocalPlayer>)>,
    mut q_stopped_piloting: RemovedComponents<Pilot>,
    q_local_player: Query<Entity, With<LocalPlayer>>,
) {
    if !q_started_flying.is_empty() {
        let Ok(mut priority) = q_hotbar_priority.single_mut() else {
            return;
        };

        priority.add(SHIP_PRIORITY_IDENTIFIER, 5);
    }

    if q_stopped_piloting.is_empty() {
        return;
    }

    let Ok(local_ent) = q_local_player.single() else {
        return;
    };

    for ent in q_stopped_piloting.read() {
        if ent == local_ent {
            let Ok(mut priority) = q_hotbar_priority.single_mut() else {
                return;
            };

            priority.remove(SHIP_PRIORITY_IDENTIFIER);
        }
    }
}

fn change_structure_system_cooldown(
    q_piloting: Query<&Pilot, With<LocalPlayer>>,
    q_systems_changed: Query<(&StructureSystem, &StructureSystemCharge), Changed<StructureSystemCharge>>,
    q_systems: Query<&StructureSystemOrdering>,
    mut q_pilot_systems_inventory: Query<&mut Inventory, With<PilotStructureSystemsInventory>>,
    mut commands: Commands,
) {
    let Ok(piloting) = q_piloting.single().map(|x| x.entity) else {
        return;
    };

    for (ss, charge) in q_systems_changed.iter() {
        if ss.structure_entity() != piloting {
            continue;
        }

        let Ok(ordering) = q_systems.get(ss.structure_entity()) else {
            continue;
        };

        let Some(order) = ordering.ordering_for(ss.id()) else {
            continue;
        };

        let Ok(mut inv) = q_pilot_systems_inventory.single_mut() else {
            continue;
        };

        inv.insert_itemstack_data(order as usize, ItemCooldown::new(1.0 - charge.0), &mut commands);
    }
}

fn sync_ship_systems(
    q_systems: Query<(&StructureSystems, &StructureSystemOrdering)>,
    q_piloting: Query<&Pilot, With<LocalPlayer>>,
    q_systems_changed: Query<(), Or<(Changed<StructureSystems>, Changed<StructureSystemOrdering>)>>,
    q_priority_changed: Query<(), (Changed<HotbarPriorityQueue>, With<LocalPlayerHotbar>)>,
    q_structure_system: Query<(&StructureSystem, Option<&StructureSystemCharge>)>,
    structure_system_types: Res<Registry<StructureSystemType>>,
    items: Res<Registry<Item>>,
    mut q_hotbar: Query<(&HotbarPriorityQueue, &mut HotbarContents), With<LocalPlayerHotbar>>,
    has_data: Res<ItemShouldHaveData>,
    mut commands: Commands,
    mut q_pilot_systems_inventory: Query<&mut Inventory, With<PilotStructureSystemsInventory>>,
) {
    let Ok(piloting) = q_piloting.single() else {
        return;
    };

    let Ok((hotbar_prio_queue, mut hotbar_contents)) = q_hotbar.single_mut() else {
        return;
    };

    if hotbar_prio_queue.active() != Some(SHIP_PRIORITY_IDENTIFIER) {
        return;
    }

    if !q_systems_changed.contains(piloting.entity) && q_priority_changed.is_empty() {
        return;
    }

    let Ok((ship_systems, systems_ordering)) = q_systems.get(piloting.entity) else {
        return;
    };

    let n_slots = hotbar_contents.n_slots();
    let mut slot = 0;

    hotbar_contents.clear_contents();

    let Ok(mut inv) = q_pilot_systems_inventory.single_mut() else {
        error!("Bad inventory!");
        return;
    };

    for system_id in systems_ordering.iter() {
        let Some(system_id) = system_id else {
            slot += 1;
            continue;
        };

        let Some(system_ent) = ship_systems.get_system_entity(system_id) else {
            slot += 1;
            continue;
        };

        let Ok((system, sys_charge)) = q_structure_system.get(system_ent) else {
            continue;
        };

        let system_type = structure_system_types.from_numeric_id(system.system_type_id().into());

        let item = items.from_numeric_id(system_type.item_icon_id());

        inv.insert_item_at(slot, item, 1, &mut commands, &has_data);
        if let Some(charge) = sys_charge {
            inv.insert_itemstack_data(slot, ItemCooldown::new(1.0 - charge.0), &mut commands);
        }

        hotbar_contents.set_itemstack_at(slot, inv.itemstack_at(slot).cloned());

        slot += 1;

        if slot == n_slots {
            break;
        }
    }
}

fn on_self_become_pilot(
    q_changed_hotbar: Query<(&HotbarPriorityQueue, &Hotbar), With<LocalPlayerHotbar>>,
    mut q_hovered_system: Query<&mut HoveredSystem, (Or<(Added<Pilot>, Added<HoveredSystem>)>, With<LocalPlayer>)>,
) {
    let Ok((queue, hotbar)) = q_changed_hotbar.single() else {
        return;
    };

    let Ok(mut selected_system) = q_hovered_system.single_mut() else {
        return;
    };

    if queue.active() != Some(SHIP_PRIORITY_IDENTIFIER) {
        return;
    }

    let selected = hotbar.selected_slot();
    selected_system.hovered_system_index = selected;
}

fn on_change_hotbar(
    q_changed_hotbar: Query<
        (&HotbarPriorityQueue, &Hotbar),
        (With<LocalPlayerHotbar>, Or<(Changed<Hotbar>, Changed<HotbarPriorityQueue>)>),
    >,
    mut q_hovered_system: Query<&mut HoveredSystem, (With<Pilot>, With<LocalPlayer>)>,
) {
    let Ok((queue, hotbar)) = q_changed_hotbar.single() else {
        return;
    };

    let Ok(mut selected_system) = q_hovered_system.single_mut() else {
        return;
    };

    if queue.active() != Some(SHIP_PRIORITY_IDENTIFIER) {
        return;
    }

    let selected = hotbar.selected_slot();
    selected_system.hovered_system_index = selected;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This set handles the player (client) changing their selected system set.
pub enum SystemSelectionSet {
    /// This set handles the player (client) changing their selected system set.
    ApplyUserChanges,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(Update, SystemSelectionSet::ApplyUserChanges);

    app.add_systems(
        Update,
        (
            add_priority_when_flying,
            (create_pilot_inventory, sync_ship_systems)
                .chain()
                .in_set(ItemStackSystemSet::CreateDataEntity),
            (
                on_self_become_pilot,
                on_change_hotbar,
                change_structure_system_cooldown,
                remove_pilot_inventory,
            )
                .chain()
                .before(SystemUsageSet::ChangeSystemBeingUsed)
                .after(SystemUsageSet::AddHoveredSlotComponent),
        )
            .in_set(SystemSelectionSet::ApplyUserChanges)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
