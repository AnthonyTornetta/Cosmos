use crate::asset::{
    asset_loading::{AllTexturesDoneLoadingEvent, AssetsDoneLoadingEvent, CosmosTextureAtlas},
    materials::animated_material::{AnimatedArrayTextureMaterial, ATTRIBUTE_PACKED_ANIMATION_DATA},
};

use super::super::*;
use serde::{Deserialize, Serialize};

#[derive(Resource)]
pub(crate) struct DefaultMaterial(pub Handle<AnimatedArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct UnlitMaterial(pub Handle<AnimatedArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct TransparentMaterial(pub Handle<AnimatedArrayTextureMaterial>);
#[derive(Resource)]
pub(crate) struct UnlitTransparentMaterial(pub Handle<AnimatedArrayTextureMaterial>);

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

        match mat.unlocalized_name() {
            "cosmos:animated" => {
                commands.entity(ev.entity).insert(match ev.material_type {
                    MaterialType::Normal => default_material.0.clone(),
                    MaterialType::Unlit => unlit_material.0.clone(),
                    MaterialType::FarAway => default_material.0.clone(),
                });
            }
            "cosmos:animated_illuminated" => {
                commands.entity(ev.entity).insert(match ev.material_type {
                    MaterialType::Normal => unlit_material.0.clone(),
                    MaterialType::Unlit => unlit_material.0.clone(),
                    MaterialType::FarAway => unlit_material.0.clone(),
                });
            }
            "cosmos:animated_transparent" => {
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
    for ev in event_reader.read() {
        commands.entity(ev.entity).remove::<Handle<AnimatedArrayTextureMaterial>>();
    }
}

fn create_main_material(image_handle: Handle<Image>, unlit: bool) -> AnimatedArrayTextureMaterial {
    AnimatedArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Mask(0.5),
        unlit,
        metallic: 0.0,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..Default::default()
    }
}

fn create_transparent_material(image_handle: Handle<Image>, unlit: bool) -> AnimatedArrayTextureMaterial {
    AnimatedArrayTextureMaterial {
        base_color_texture: Some(image_handle),
        alpha_mode: AlphaMode::Add,
        unlit,
        metallic: 0.0,
        reflectance: 0.0,
        perceptual_roughness: 1.0,
        ..Default::default()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct AnimationData {
    pub frame_duration_ms: u16,
    pub n_frames: u16,
}

#[derive(Default, Clone)]
struct AnimatedMaterialInformationGenerator {
    mapping: HashMap<u16, u32>,
}

impl AnimatedMaterialInformationGenerator {
    pub fn add_block_animation_data(&mut self, block_id: u16, data: AnimationData) {
        let packed: u32 = ((data.frame_duration_ms as u32) << 16) | (data.n_frames as u32);

        self.mapping.insert(block_id, packed);
    }
}

impl MaterialMeshInformationGenerator for AnimatedMaterialInformationGenerator {
    fn generate_information(&self, block_id: u16, mesh_info: &MeshInformation) -> Vec<(MeshVertexAttribute, VertexAttributeValues)> {
        let packed = *self
            .mapping
            .get(&block_id)
            .unwrap_or_else(|| panic!("Missing animation data for block {block_id}"));

        let animation_data = (0..mesh_info.positions.len()).map(|_| packed).collect::<Vec<u32>>();

        vec![(ATTRIBUTE_PACKED_ANIMATION_DATA, animation_data.into())]
    }

    fn add_information(&mut self, block_id: u16, additional_information: &HashMap<String, String>) {
        self.add_block_animation_data(
            block_id,
            AnimationData {
                frame_duration_ms: additional_information
                    .get("frame_duration_ms")
                    .expect("Missing 'frame_duration_ms' for animated material! Please add that to your json file.")
                    .parse()
                    .expect("Invalid 'frame_duration_ms' value. It must be a number between 0 and 65535"),
                n_frames: additional_information
                    .get("n_frames")
                    .expect("Missing 'n_frames' for animated material! Please add that to your json file.")
                    .parse()
                    .expect("Invalid 'n_frames' value. It must be a number between 0 and 65535"),
            },
        );
    }
}

fn create_materials(
    mut commands: Commands,
    mut material_registry: ResMut<Registry<MaterialDefinition>>,
    mut materials: ResMut<Assets<AnimatedArrayTextureMaterial>>,
    event_reader: EventReader<AllTexturesDoneLoadingEvent>,
    mut event_writer: EventWriter<AssetsDoneLoadingEvent>,
    texture_atlases: Res<Registry<CosmosTextureAtlas>>,
) {
    if !event_reader.is_empty() {
        if let Some(atlas) = texture_atlases.from_id("cosmos:main") {
            let unlit_default_material = materials.add(create_main_material(atlas.texture_atlas.get_atlas_handle().clone(), true));
            let default_material = materials.add(create_main_material(atlas.texture_atlas.get_atlas_handle().clone(), false));
            let transparent_material = materials.add(create_transparent_material(atlas.texture_atlas.get_atlas_handle().clone(), false));
            let unlit_transparent_material =
                materials.add(create_transparent_material(atlas.texture_atlas.get_atlas_handle().clone(), true));

            commands.insert_resource(DefaultMaterial(default_material));
            commands.insert_resource(UnlitMaterial(unlit_default_material));
            commands.insert_resource(TransparentMaterial(transparent_material));
            commands.insert_resource(UnlitTransparentMaterial(unlit_transparent_material));

            material_registry.register(MaterialDefinition::new(
                "cosmos:animated",
                Some(Box::<AnimatedMaterialInformationGenerator>::default()),
            ));
            material_registry.register(MaterialDefinition::new(
                "cosmos:animated_illuminated",
                Some(Box::<AnimatedMaterialInformationGenerator>::default()),
            ));
            material_registry.register(MaterialDefinition::new(
                "cosmos:animated_transparent",
                Some(Box::<AnimatedMaterialInformationGenerator>::default()),
            ));
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
