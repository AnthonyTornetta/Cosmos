//! Blueprint support for structures docked to the blueprinted root structure.

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use cosmos_core::{
    block::Block,
    physics::location::Location,
    registry::Registry,
    structure::{
        Structure,
        blueprint::{BlueprintType, COMPOSITE_BLUEPRINT_DATA_KEY, CompositeBlueprint, CompositeBlueprintChild, CompositeBlueprintDocked},
        loading::StructureLoadingSet,
        systems::dock_system::Docked,
    },
};

use crate::{
    persistence::{
        SerializedData,
        loading::{LOADING_SCHEDULE, LoadingBlueprintSystemSet, NeedsBlueprintLoaded},
        saving::{BlueprintingSystemSet, NeedsBlueprinted, SAVING_SCHEDULE},
    },
    structure::{
        persistence::{chunk::BlockDataBlueprintingSet, save_structure},
        systems::DockedEntities,
    },
};

#[derive(Component)]
struct CompositeBlueprintChildSave {
    root: Entity,
    index: u32,
    parent_index: u32,
    blueprint_type: BlueprintType,
    docked: CompositeBlueprintDocked,
}

#[derive(Component)]
struct CompositeBlueprintChildrenSpawned;

#[derive(Component, Clone, Copy)]
struct BlueprintLoadedStructure {
    root: Entity,
    index: u32,
}

#[derive(Component)]
struct PendingBlueprintDock {
    root: Entity,
    parent_index: u32,
    docked: CompositeBlueprintDocked,
}

fn stage_composite_blueprint_children(
    q_roots: Query<Entity, With<NeedsBlueprinted>>,
    q_docked_entities: Query<&DockedEntities>,
    q_structures: Query<(
        &Structure,
        Option<&Docked>,
        Has<cosmos_core::prelude::Ship>,
        Has<cosmos_core::prelude::Station>,
        Has<SerializedData>,
    )>,
    mut commands: Commands,
    blocks: Res<Registry<Block>>,
) {
    for root in &q_roots {
        let mut seen = HashSet::new();
        seen.insert(root);

        let mut stack = q_docked_entities
            .get(root)
            .map(|docked| docked.iter().map(|child| (child, 0)).collect::<Vec<_>>())
            .unwrap_or_default();

        let mut indices = HashMap::from([(root, 0_u32)]);
        let mut next_index = 1_u32;

        while let Some((entity, traversal_parent_index)) = stack.pop() {
            if !seen.insert(entity) {
                continue;
            }

            let Ok((structure, docked, is_ship, is_station, has_serialized_data)) = q_structures.get(entity) else {
                warn!("Skipping docked blueprint child {entity:?}: missing structure data.");
                continue;
            };

            if has_serialized_data {
                warn!("Skipping docked blueprint child {entity:?}: it is already being serialized.");
                continue;
            }

            let Some(docked) = docked else {
                warn!("Skipping docked blueprint child {entity:?}: missing Docked component.");
                continue;
            };

            let blueprint_type = if is_ship {
                BlueprintType::Ship
            } else if is_station {
                BlueprintType::Station
            } else {
                warn!("Skipping docked blueprint child {entity:?}: only ship/station children are supported.");
                continue;
            };

            let index = next_index;
            next_index += 1;
            indices.insert(entity, index);

            let parent_index = indices.get(&docked.to).copied().unwrap_or(traversal_parent_index);

            let mut serialized_data = SerializedData::default();
            save_structure(structure, &mut serialized_data, &blocks, &mut commands);
            match blueprint_type {
                BlueprintType::Ship => serialized_data.serialize_data("cosmos:is_ship", &true),
                BlueprintType::Station => serialized_data.serialize_data("cosmos:is_station", &true),
                BlueprintType::Asteroid => unreachable!("Asteroid children are filtered out above."),
            }

            commands.entity(entity).insert((
                serialized_data,
                CompositeBlueprintChildSave {
                    root,
                    index,
                    parent_index,
                    blueprint_type,
                    docked: CompositeBlueprintDocked {
                        to_block: docked.to_block,
                        this_block: docked.this_block,
                        relative_rotation: docked.relative_rotation,
                        relative_translation: docked.relative_translation,
                        rotate_x: docked.rotate_x,
                        rotate_y: docked.rotate_y,
                        rotate_z: docked.rotate_z,
                        parent_anchor: docked.parent_anchor,
                        child_anchor: docked.child_anchor,
                    },
                },
            ));

            if let Ok(docked_entities) = q_docked_entities.get(entity) {
                stack.extend(docked_entities.iter().map(|child| (child, index)));
            }
        }
    }
}

fn finalize_composite_blueprint_data(
    mut q_roots: Query<&mut SerializedData, With<NeedsBlueprinted>>,
    q_children: Query<(Entity, &SerializedData, &CompositeBlueprintChildSave), Without<NeedsBlueprinted>>,
    mut commands: Commands,
) {
    let mut by_root = HashMap::<Entity, Vec<CompositeBlueprintChild>>::default();

    for (entity, serialized_data, child) in &q_children {
        by_root.entry(child.root).or_default().push(CompositeBlueprintChild {
            index: child.index,
            parent_index: child.parent_index,
            blueprint_type: child.blueprint_type,
            serialized_data: serialized_data.save_data().clone(),
            docked: child.docked.clone(),
        });

        commands
            .entity(entity)
            .remove::<SerializedData>()
            .remove::<CompositeBlueprintChildSave>();
    }

    for (root, mut children) in by_root {
        let Ok(mut root_data) = q_roots.get_mut(root) else {
            continue;
        };

        children.sort_by_key(|child| child.index);
        root_data.serialize_data(COMPOSITE_BLUEPRINT_DATA_KEY, &CompositeBlueprint { children });
    }
}

fn spawn_composite_blueprint_children(
    q_roots: Query<(Entity, &SerializedData, &NeedsBlueprintLoaded), Without<CompositeBlueprintChildrenSpawned>>,
    mut commands: Commands,
) {
    for (root, serialized_data, needs_blueprint_loaded) in &q_roots {
        let Ok(composite) = serialized_data.deserialize_data::<CompositeBlueprint>(COMPOSITE_BLUEPRINT_DATA_KEY) else {
            continue;
        };

        commands
            .entity(root)
            .insert((CompositeBlueprintChildrenSpawned, BlueprintLoadedStructure { root, index: 0 }));

        let mut transforms =
            HashMap::<u32, (Location, Quat)>::from([(0, (needs_blueprint_loaded.spawn_at, needs_blueprint_loaded.rotation))]);
        let mut children = composite.children;
        children.sort_by_key(|child| child.index);

        for child in children {
            let Some((parent_location, parent_rotation)) = transforms.get(&child.parent_index).copied() else {
                warn!(
                    "Skipping composite blueprint child {}: parent {} was not found.",
                    child.index, child.parent_index
                );
                continue;
            };

            let rotation = parent_rotation * child.docked.relative_rotation;
            let location = parent_location + parent_rotation.mul_vec3(child.docked.relative_translation);

            transforms.insert(child.index, (location, rotation));

            commands.spawn((
                SerializedData::from_save_data(child.serialized_data),
                NeedsBlueprintLoaded {
                    path: String::new(),
                    spawn_at: location,
                    rotation,
                },
                BlueprintLoadedStructure { root, index: child.index },
                PendingBlueprintDock {
                    root,
                    parent_index: child.parent_index,
                    docked: child.docked,
                },
            ));
        }
    }
}

fn resolve_pending_blueprint_docks(
    mut commands: Commands,
    q_pending: Query<(Entity, &PendingBlueprintDock), With<Structure>>,
    q_loaded: Query<(Entity, &BlueprintLoadedStructure), With<Structure>>,
) {
    let loaded = q_loaded
        .iter()
        .map(|(entity, loaded)| ((loaded.root, loaded.index), entity))
        .collect::<HashMap<_, _>>();

    for (entity, pending) in &q_pending {
        let Some(parent) = loaded.get(&(pending.root, pending.parent_index)).copied() else {
            continue;
        };

        commands
            .entity(entity)
            .insert(Docked {
                to: parent,
                to_block: pending.docked.to_block,
                this_block: pending.docked.this_block,
                relative_rotation: pending.docked.relative_rotation,
                relative_translation: pending.docked.relative_translation,
                rotate_x: pending.docked.rotate_x,
                rotate_y: pending.docked.rotate_y,
                rotate_z: pending.docked.rotate_z,
                parent_anchor: pending.docked.parent_anchor,
                child_anchor: pending.docked.child_anchor,
            })
            .remove::<PendingBlueprintDock>();
    }
}

fn cleanup_composite_blueprint_load_markers(
    mut commands: Commands,
    q_pending: Query<&PendingBlueprintDock>,
    q_loaded: Query<(Entity, &BlueprintLoadedStructure)>,
) {
    let pending_roots = q_pending.iter().map(|pending| pending.root).collect::<HashSet<_>>();

    for (entity, loaded) in &q_loaded {
        if pending_roots.contains(&loaded.root) {
            continue;
        }

        commands
            .entity(entity)
            .remove::<BlueprintLoadedStructure>()
            .remove::<CompositeBlueprintChildrenSpawned>();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        (
            stage_composite_blueprint_children.in_set(BlueprintingSystemSet::DoBlueprinting),
            finalize_composite_blueprint_data
                .in_set(BlueprintingSystemSet::FinalizeBlueprintData)
                .after(BlockDataBlueprintingSet::DoneBlueprintingBlockData),
        ),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            spawn_composite_blueprint_children
                .in_set(LoadingBlueprintSystemSet::DoLoadingBlueprints)
                .before(StructureLoadingSet::LoadStructure),
            (resolve_pending_blueprint_docks, cleanup_composite_blueprint_load_markers)
                .chain()
                .in_set(LoadingBlueprintSystemSet::FinalizeLoadingBlueprints),
        ),
    );
}
