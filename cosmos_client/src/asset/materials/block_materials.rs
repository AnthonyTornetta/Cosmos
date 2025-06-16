//! The material used by most blocks

use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef, VertexFormat};
use bevy::render::render_resource::{AsBindGroup, RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError};

/// Material used for standard, non-animated blocks.
pub type ArrayTextureMaterial = ExtendedMaterial<StandardMaterial, ArrayTextureMaterialExtension>;

/// Specifies the texture index to use
pub const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    // A "high" random id should be used for custom attributes to ensure consistent sorting and avoid collisions with other attributes.
    // See the MeshVertexAttribute docs for more info.
    MeshVertexAttribute::new("ArrayTextureIndex", 923840841, VertexFormat::Uint32);

/// Material used for standard, non-animated blocks - extends [`StandardMaterial`]. Use
/// [`ArrayTextureMaterial`] to reference the extension with the [`StandardMaterial`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct ArrayTextureMaterialExtension {
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

impl MaterialExtension for ArrayTextureMaterialExtension {
    fn vertex_shader() -> ShaderRef {
        "cosmos/shaders/block.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "cosmos/shaders/block.wgsl".into()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        "cosmos/shaders/block_prepass.wgsl".into()
    }
    fn prepass_fragment_shader() -> ShaderRef {
        "cosmos/shaders/block_prepass.wgsl".into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(20),
        ])?;

        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<ArrayTextureMaterial>::default());
}
