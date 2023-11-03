use crate::asset::asset_loading::{AllTexturesDoneLoadingEvent, AssetsDoneLoadingEvent, CosmosTextureAtlas};

use super::super::{block_materials::ArrayTextureMaterial, *};
use bevy::prelude::Commands;

#[derive(Resource)]
pub(crate) struct DefaultMaterial(pub Handle<ArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct UnlitMaterial(pub Handle<ArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct TransparentMaterial(pub Handle<ArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct UnlitTransparentMaterial(pub Handle<ArrayTextureMaterial>);

fn respond_to_add_materials_event(
    material_registry: Res<Registry<MaterialDefinition>>,
    mut commands: Commands,
    mut event_reader: EventReader<AddMaterialEvent>,

    default_material: Res<DefaultMaterial>,
    unlit_material: Res<UnlitMaterial>,
    transparent_material: Res<TransparentMaterial>,
    unlit_transparent_material: Res<UnlitTransparentMaterial>,
) {
    for ev in event_reader.iter() {
        let mat = material_registry.from_numeric_id(ev.add_material_id);

        match mat.unlocalized_name() {
            "cosmos:main" => {
                commands.entity(ev.entity).insert(match ev.material_type {
                    MaterialType::Normal => default_material.0.clone(),
                    MaterialType::Unlit => unlit_material.0.clone(),
                    MaterialType::FarAway => default_material.0.clone(),
                });
            }
            "cosmos:illuminated" => {
                commands.entity(ev.entity).insert(match ev.material_type {
                    MaterialType::Normal => unlit_material.0.clone(),
                    MaterialType::Unlit => unlit_material.0.clone(),
                    MaterialType::FarAway => unlit_material.0.clone(),
                });
            }
            "cosmos:transparent" => {
                commands.entity(ev.entity).insert(match ev.material_type {
                    MaterialType::Normal => transparent_material.0.clone(),
                    MaterialType::Unlit => unlit_transparent_material.0.clone(),
                    MaterialType::FarAway => default_material.0.clone(),
                });
            }
            _ => {}
        }
    }
}

fn respond_to_remove_materails_event(mut event_reader: EventReader<RemoveAllMaterialsEvent>, mut commands: Commands) {
    for ev in event_reader.iter() {
        commands.entity(ev.entity).remove::<Handle<ArrayTextureMaterial>>();
    }
}

fn create_main_material(image_handle: Handle<Image>, unlit: bool) -> ArrayTextureMaterial {
    ArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit,
        metallic: 0.0,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..Default::default()
    }
}

fn create_transparent_material(image_handle: Handle<Image>, unlit: bool) -> ArrayTextureMaterial {
    ArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Add,
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
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
    event_reader: EventReader<AllTexturesDoneLoadingEvent>,
    mut event_writer: EventWriter<AssetsDoneLoadingEvent>,
    texture_atlases: Res<Registry<CosmosTextureAtlas>>,
) {
    if !event_reader.is_empty() {
        if let Some(atlas) = texture_atlases.from_id("cosmos:main") {
            let default_material = materials.add(create_main_material(atlas.texture_atlas.texture.clone(), false));
            let unlit_default_material = materials.add(create_main_material(atlas.texture_atlas.texture.clone(), true));
            let transparent_material = materials.add(create_transparent_material(atlas.texture_atlas.texture.clone(), false));
            let unlit_transparent_material = materials.add(create_transparent_material(atlas.texture_atlas.texture.clone(), true));

            commands.insert_resource(DefaultMaterial(default_material));
            commands.insert_resource(UnlitMaterial(unlit_default_material));
            commands.insert_resource(TransparentMaterial(transparent_material));
            commands.insert_resource(UnlitTransparentMaterial(unlit_transparent_material));

            material_registry.register(MaterialDefinition::new("cosmos:main"));
            material_registry.register(MaterialDefinition::new("cosmos:illuminated"));
            material_registry.register(MaterialDefinition::new("cosmos:transparent"));
        }

        event_writer.send(AssetsDoneLoadingEvent);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            respond_to_remove_materails_event.after(remove_materials).before(add_materials),
            respond_to_add_materials_event.after(add_materials),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(Update, (create_materials,).run_if(in_state(GameState::PostLoading)));
}
