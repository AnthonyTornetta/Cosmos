use lod_materials::LodArrayTextureMaterial;

use crate::asset::asset_loading::{AllTexturesDoneLoadingEvent, AssetsDoneLoadingEvent, CosmosTextureAtlas};

use super::super::*;

#[derive(Resource)]
pub(crate) struct DefaultMaterial(pub Vec<Handle<LodArrayTextureMaterial>>);
#[derive(Resource)]
pub(crate) struct UnlitMaterial(pub Vec<Handle<LodArrayTextureMaterial>>);
#[derive(Resource)]
pub(crate) struct TransparentMaterial(pub Vec<Handle<LodArrayTextureMaterial>>);
#[derive(Resource)]
pub(crate) struct UnlitTransparentMaterial(pub Vec<Handle<LodArrayTextureMaterial>>);

fn respond_to_add_materials_event(
    material_registry: Res<Registry<MaterialDefinition>>,
    mut commands: Commands,
    mut event_reader: EventReader<AddMaterialEvent>,

    default_material: Res<DefaultMaterial>,
    unlit_material: Res<UnlitMaterial>,
    transparent_material: Res<TransparentMaterial>,
    unlit_transparent_material: Res<UnlitTransparentMaterial>,
) {
    for ev in event_reader.read() {
        let mat = material_registry.from_numeric_id(ev.add_material_id);

        let idx = ev.texture_dimensions_index as usize;

        match mat.unlocalized_name() {
            "cosmos:lod" => {
                commands.entity(ev.entity).insert(MeshMaterial3d(match ev.material_type {
                    MaterialType::Normal => default_material.0[idx].clone_weak(),
                    MaterialType::Illuminated => unlit_material.0[idx].clone_weak(),
                    MaterialType::FarAway => default_material.0[idx].clone_weak(),
                }));
            }
            "cosmos:lod_illuminated" => {
                commands.entity(ev.entity).insert(MeshMaterial3d(match ev.material_type {
                    MaterialType::Normal => unlit_material.0[idx].clone_weak(),
                    MaterialType::Illuminated => unlit_material.0[idx].clone_weak(),
                    MaterialType::FarAway => unlit_material.0[idx].clone_weak(),
                }));
            }
            "cosmos:lod_transparent" => {
                commands.entity(ev.entity).insert(MeshMaterial3d(match ev.material_type {
                    MaterialType::Normal => transparent_material.0[idx].clone_weak(),
                    MaterialType::Illuminated => unlit_transparent_material.0[idx].clone_weak(),
                    MaterialType::FarAway => default_material.0[idx].clone_weak(),
                }));
            }
            _ => {}
        }
    }
}

fn respond_to_remove_materails_event(mut event_reader: EventReader<RemoveAllMaterialsEvent>, mut commands: Commands) {
    for ev in event_reader.read() {
        commands.entity(ev.entity).remove::<MeshMaterial3d<LodArrayTextureMaterial>>();
    }
}

fn create_main_material(image_handle: Handle<Image>, unlit: bool) -> LodArrayTextureMaterial {
    LodArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit,
        metallic: 0.0,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..Default::default()
    }
}

fn create_transparent_material(image_handle: Handle<Image>, unlit: bool) -> LodArrayTextureMaterial {
    LodArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Blend,
        unlit,
        metallic: 0.0,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..Default::default()
    }
}

fn create_materials(
    mut commands: Commands,
    mut material_registry: ResMut<Registry<MaterialDefinition>>,
    mut materials: ResMut<Assets<LodArrayTextureMaterial>>,
    event_reader: EventReader<AllTexturesDoneLoadingEvent>,
    mut event_writer: EventWriter<AssetsDoneLoadingEvent>,
    texture_atlases: Res<Registry<CosmosTextureAtlas>>,
) {
    if !event_reader.is_empty() {
        if let Some(atlas) = texture_atlases.from_id("cosmos:main") {
            let mut default_material = vec![];
            let mut unlit_default_material = vec![];
            let mut transparent_material = vec![];
            let mut unlit_transparent_material = vec![];

            for dimension_atlas in atlas.texture_atlases() {
                default_material.push(materials.add(create_main_material(dimension_atlas.get_atlas_handle().clone(), false)));
                unlit_default_material.push(materials.add(create_main_material(dimension_atlas.get_atlas_handle().clone(), true)));
                transparent_material.push(materials.add(create_transparent_material(dimension_atlas.get_atlas_handle().clone(), false)));
                unlit_transparent_material
                    .push(materials.add(create_transparent_material(dimension_atlas.get_atlas_handle().clone(), true)));
            }

            commands.insert_resource(DefaultMaterial(default_material));
            commands.insert_resource(UnlitMaterial(unlit_default_material));
            commands.insert_resource(TransparentMaterial(transparent_material));
            commands.insert_resource(UnlitTransparentMaterial(unlit_transparent_material));

            material_registry.register(MaterialDefinition::new("cosmos:lod", None));
            material_registry.register(MaterialDefinition::new("cosmos:lod_illuminated", None));
            material_registry.register(MaterialDefinition::new("cosmos:lod_transparent", None));
        }

        event_writer.send(AssetsDoneLoadingEvent);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            respond_to_remove_materails_event.in_set(MaterialsSystemSet::ProcessRemoveMaterialsEvents),
            respond_to_add_materials_event.in_set(MaterialsSystemSet::ProcessAddMaterialsEvents),
        ),
    )
    .add_systems(
        Update,
        (create_materials,)
            .in_set(AssetsSet::AssetsLoading)
            .ambiguous_with(AssetsSet::AssetsLoading)
            .run_if(in_state(GameState::PostLoading)),
    );
}
