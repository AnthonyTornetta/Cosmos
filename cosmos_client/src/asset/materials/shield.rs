//! Contains the materials for the client rendering of shields.

use bevy::{
    app::App,
    asset::Asset,
    math::Vec4,
    pbr::{ExtendedMaterial, MaterialExtension, MaterialPlugin, StandardMaterial},
    reflect::TypePath,
    render::{mesh::MeshVertexBufferLayoutRef, render_resource::AsBindGroup},
};
use bevy_app_compute::prelude::ShaderRef;

/// The maximum number of shield hits that can be rendered
pub const MAX_SHIELD_HIT_POINTS: usize = 100;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
/// The Material responsible for shield rendering
pub struct ShieldMaterialExtension {
    #[uniform(100)]
    /// Controls the ripple animation of the shield when hit
    /// Vector format: (normal x, normal y, normal z, time since start)
    pub ripples: [Vec4; MAX_SHIELD_HIT_POINTS],
}

impl MaterialExtension for ShieldMaterialExtension {
    fn fragment_shader() -> ShaderRef {
        "cosmos/shaders/shield.wgsl".into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        "cosmos/shaders/shield.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialExtensionKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;

        Ok(())
    }
}

/// The Material responsible for shield rendering
pub type ShieldMaterial = ExtendedMaterial<StandardMaterial, ShieldMaterialExtension>;

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ShieldMaterial>::default());
}
