//! Displays the ships's system selection hotbar

use bevy::prelude::*;
use cosmos_core::{
    inventory::itemstack::{ItemShouldHaveData, ItemStack, ItemStackSystemSet},
    item::Item,
    netty::client::LocalPlayer,
    registry::Registry,
    state::GameState,
    structure::{
        ship::pilot::Pilot,
        systems::{StructureSystem, StructureSystemType, StructureSystems},
    },
};

use crate::{
    structure::systems::player_interactions::{HoveredSystem, SystemUsageSet},
    ui::hotbar::{Hotbar, HotbarContents, HotbarPriorityQueue, LocalPlayerHotbar},
};

const SHIP_PRIORITY_IDENTIFIER: &str = "cosmos:ship_systems";

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

fn sync_ship_systems(
    q_systems: Query<&StructureSystems>,
    q_piloting: Query<&Pilot, With<LocalPlayer>>,
    q_systems_changed: Query<(), Changed<StructureSystems>>,
    q_priority_changed: Query<(), (Changed<HotbarPriorityQueue>, With<LocalPlayerHotbar>)>,
    q_structure_system: Query<&StructureSystem>,
    structure_system_types: Res<Registry<StructureSystemType>>,
    items: Res<Registry<Item>>,
    mut q_hotbar: Query<(&HotbarPriorityQueue, &mut HotbarContents), With<LocalPlayerHotbar>>,
    has_data: Res<ItemShouldHaveData>,
    mut commands: Commands,
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

    let Ok(ship_systems) = q_systems.get(piloting.entity) else {
        return;
    };

    let n_slots = hotbar_contents.n_slots();
    let mut slot = 0;

    hotbar_contents.clear_contents(Some(&mut commands));

    for system_ent in ship_systems.all_activatable_systems() {
        let Ok(system) = q_structure_system.get(system_ent) else {
            continue;
        };

        let system_type = structure_system_types.from_numeric_id(system.system_type_id().into());

        let item = items.from_numeric_id(system_type.item_icon_id());

        hotbar_contents.set_itemstack_at(
            slot,
            Some(ItemStack::with_quantity(
                item,
                1,
                // TODO: Make this hotbar use an actual inventory so this isn't meaningless
                (Entity::PLACEHOLDER, 0),
                &mut commands,
                &has_data,
            )),
        );

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
            sync_ship_systems.in_set(ItemStackSystemSet::CreateDataEntity),
            (on_self_become_pilot, on_change_hotbar)
                .chain()
                .before(SystemUsageSet::ChangeSystemBeingUsed)
                .after(SystemUsageSet::AddHoveredSlotComponent),
        )
            .in_set(SystemSelectionSet::ApplyUserChanges)
            .chain()
            .run_if(in_state(GameState::Playing)),
    );
}
