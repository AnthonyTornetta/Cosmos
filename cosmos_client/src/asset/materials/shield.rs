use bevy::{
    app::App,
    asset::Asset,
    math::Vec4,
    pbr::{AlphaMode, ExtendedMaterial, Material, MaterialExtension, MaterialPlugin, StandardMaterial},
    reflect::TypePath,
    render::{color::Color, render_resource::AsBindGroup},
};

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ShieldMaterialExtension {
    #[uniform(100)]
    pub ripples: [Vec4; 20],
}

impl MaterialExtension for ShieldMaterialExtension {
    fn fragment_shader() -> bevy_app_compute::prelude::ShaderRef {
        "cosmos/shaders/shield.wgsl".into()
    }

    fn deferred_fragment_shader() -> bevy_app_compute::prelude::ShaderRef {
        "cosmos/shaders/shield.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayout,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;

        Ok(())
    }
}

pub type ShieldMaterial = ExtendedMaterial<StandardMaterial, ShieldMaterialExtension>;

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ShieldMaterial>::default());
}
