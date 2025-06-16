//! The material used by most blocks

use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexFormat};
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError};

/// Material used for standard, non-animated blocks.
pub type LodArrayTextureMaterial = ExtendedMaterial<StandardMaterial, LodArrayTextureMaterialExtension>;

/// The direction the vertex normal is facing in.
/// Use in conjunction with [`Mesh::insert_attribute`] or [`Mesh::with_inserted_attribute`].
pub const ATTRIBUTE_PACKED_DATA: MeshVertexAttribute = MeshVertexAttribute::new("PackedData", 923840840, VertexFormat::Uint32);

/// Material used for standard, non-animated blocks - extends [`StandardMaterial`]. Use
/// [`ArrayTextureMaterial`] to reference the extension with the [`StandardMaterial`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct LodArrayTextureMaterialExtension {
    /// The texture component of the material's color before lighting.
    /// The actual pre-lighting color is `base_color * this_texture`.
    ///
    /// See [`base_color`] for details.
    ///
    /// You should set `base_color` to [`Color::WHITE`] (the default)
    /// if you want the texture to show as-is.
    ///
    /// Setting `base_color` to something else than white will tint
    /// the texture. For example, setting `base_color` to pure red will
    /// tint the texture red.
    ///
    /// [`base_color`]: AnimatedArrayTextureMaterial::base_color
    #[texture(101, dimension = "2d_array")]
    #[sampler(102)]
    #[dependency]
    pub base_color_texture: Option<Handle<Image>>,
}

impl MaterialExtension for LodArrayTextureMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "cosmos/shaders/lod.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "cosmos/shaders/lod.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            ATTRIBUTE_PACKED_DATA.at_shader_location(1),
        ])?;

        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<LodArrayTextureMaterial>::default());
}
