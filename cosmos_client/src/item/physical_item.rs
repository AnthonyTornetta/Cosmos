//! Renders physical items in the client

use bevy::prelude::*;
use cosmos_core::{
    inventory::Inventory,
    item::{Item, physical_item::PhysicalItem},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

use crate::asset::materials::{AddMaterialEvent, MaterialType, MaterialsSystemSet, RemoveAllMaterialsEvent};

use super::item_mesh::ItemMeshMaterial;

fn render_physical_item(
    mut commands: Commands,
    mut evw_add_material: EventWriter<AddMaterialEvent>,
    mut evw_remove_material: EventWriter<RemoveAllMaterialsEvent>,
    items: Res<Registry<Item>>,
    item_rendering_info: Res<Registry<ItemMeshMaterial>>,
    mut q_physical_item: Query<
        (Entity, &mut Transform, &Inventory),
        (
            Or<(Changed<Inventory>, Changed<PhysicalItem>, Added<Transform>)>,
            With<PhysicalItem>,
        ),
    >,
) {
    for (ent, mut trans, inventory) in q_physical_item.iter_mut() {
        let mut ecmds = commands.entity(ent);
        let Some(is) = inventory.itemstack_at(0) else {
            ecmds.remove::<Mesh3d>();
            continue;
        };

        let Some(rendering_info) = item_rendering_info.from_id(items.from_numeric_id(is.item_id()).unlocalized_name()) else {
            ecmds.remove::<Mesh3d>();
            continue;
        };

        trans.scale = Vec3::splat(0.2);

        ecmds.insert((Visibility::default(), Mesh3d(rendering_info.mesh_handle().clone_weak())));
        evw_remove_material.write(RemoveAllMaterialsEvent { entity: ent });
        evw_add_material.write(AddMaterialEvent {
            entity: ent,
            add_material_id: rendering_info.material_id(),
            material_type: MaterialType::Normal,
            texture_dimensions_index: rendering_info.texture_dimension_index(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        render_physical_item
            .run_if(in_state(GameState::Playing))
            .in_set(MaterialsSystemSet::RequestMaterialChanges),
    );
}
