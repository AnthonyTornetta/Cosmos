//! Renders physical items in the client

use bevy::{
    app::Update,
    asset::Handle,
    math::Vec3,
    prelude::{
        in_state, App, Changed, Commands, Entity, EventWriter, IntoSystemConfigs, Mesh, Query, Res, Transform, VisibilityBundle, With,
    },
};
use cosmos_core::{
    inventory::Inventory,
    item::{physical_item::PhysicalItem, Item},
    netty::system_sets::NetworkingSystemsSet,
    registry::{identifiable::Identifiable, Registry},
};

use crate::{
    asset::materials::{AddMaterialEvent, MaterialType, MaterialsSystemSet, RemoveAllMaterialsEvent},
    state::game_state::GameState,
};

use super::item_mesh::ItemMeshMaterial;

fn render_physical_item(
    mut commands: Commands,
    mut evw_add_material: EventWriter<AddMaterialEvent>,
    mut evw_remove_material: EventWriter<RemoveAllMaterialsEvent>,
    items: Res<Registry<Item>>,
    item_rendering_info: Res<Registry<ItemMeshMaterial>>,
    mut q_physical_item: Query<(Entity, &mut Transform, &Inventory), (Changed<Inventory>, With<PhysicalItem>)>,
) {
    for (ent, mut trans, inventory) in q_physical_item.iter_mut() {
        let mut ecmds = commands.entity(ent);
        let Some(is) = inventory.itemstack_at(0) else {
            ecmds.remove::<Handle<Mesh>>();
            continue;
        };

        let Some(rendering_info) = item_rendering_info.from_id(items.from_numeric_id(is.item_id()).unlocalized_name()) else {
            ecmds.remove::<Handle<Mesh>>();
            continue;
        };

        trans.scale = Vec3::splat(0.2);

        ecmds.insert((VisibilityBundle::default(), rendering_info.mesh_handle().clone_weak()));
        evw_remove_material.send(RemoveAllMaterialsEvent { entity: ent });
        evw_add_material.send(AddMaterialEvent {
            entity: ent,
            add_material_id: rendering_info.material_id(),
            material_type: MaterialType::Normal,
            texture_dimensions_index: rendering_info.texture_dimension_index(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        render_physical_item
            .run_if(in_state(GameState::Playing))
            .in_set(MaterialsSystemSet::RequestMaterialChanges)
            .in_set(NetworkingSystemsSet::Between),
    );
}