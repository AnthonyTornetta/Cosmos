//! The material used by most blocks

use bevy::{
    math::{Affine2, Affine3, vec2},
    pbr::{MaterialPipeline, MaterialPipelineKey, OpaqueRendererMethod, deferred::DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID},
    prelude::{Asset, Reflect},
    render::{
        mesh::{MeshVertexAttribute, MeshVertexBufferLayoutRef},
        render_asset::RenderAssets,
        render_resource::{
            AsBindGroup, AsBindGroupShaderType, Face, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError,
            TextureFormat, VertexFormat,
        },
        texture::GpuImage,
    },
};
use bitflags::bitflags;

use crate::*;

/// Specifies the animation data to use
pub const ATTRIBUTE_PACKED_ANIMATION_DATA: MeshVertexAttribute =
    // A "high" random id should be used for custom attributes to ensure consistent sorting and avoid collisions with other attributes.
    // See the MeshVertexAttribute docs for more info.
    MeshVertexAttribute::new("AnimationData", 2212350841, VertexFormat::Uint32);

/// Specifies the texture index to use
pub const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute =
    // A "high" random id should be used for custom attributes to ensure consistent sorting and avoid collisions with other attributes.
    // See the MeshVertexAttribute docs for more info.
    MeshVertexAttribute::new("ArrayTextureIndex", 923840841, VertexFormat::Uint32);

/// An enum to define which UV attribute to use for a texture.
///
/// It is used for every texture in the [`AnimatedArrayTextureMaterial`].
/// It only supports two UV attributes, [`Mesh::ATTRIBUTE_UV_0`] and [`Mesh::ATTRIBUTE_UV_1`].
/// The default is [`UvChannel::Uv0`].
#[derive(Reflect, Default, Debug, Clone, PartialEq, Eq)]
#[reflect(Default, Debug)]
pub enum UvChannel {
    #[default]
    /// From [`bevy::prelude::StandardMaterial`]
    Uv0,
    /// From [`bevy::prelude::StandardMaterial`]
    Uv1,
}

/// A material with "standard" properties used in PBR lighting
/// Standard property values with pictures here
/// <https://google.github.io/filament/Material%20Properties.pdf>.
///
/// May be created directly from a [`Color`] or an [`Image`].
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(BlockMaterialKey)]
#[uniform(0, AnimatedArrayTextureMaterialUniform)]
#[reflect(Default, Debug)]
pub struct AnimatedArrayTextureMaterial {
    /// The color of the surface of the material before lighting.
    ///
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between. If used together with a `base_color_texture`, this is factored into the final
    /// base color as `base_color * base_color_texture_value`
    ///
    /// Defaults to [`Color::WHITE`].
    pub base_color: Color,

    /// The UV channel to use for the [`AnimatedArrayTextureMaterial::base_color_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub base_color_channel: UvChannel,

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
    #[texture(1, dimension = "2d_array")]
    #[sampler(2)]
    #[dependency]
    pub base_color_texture: Option<Handle<Image>>,

    // Use a color for user friendliness even though we technically don't use the alpha channel
    // Might be used in the future for exposure correction in HDR
    /// Color the material "emits" to the camera.
    ///
    /// This is typically used for monitor screens or LED lights.
    /// Anything that can be visible even in darkness.
    ///
    /// The emissive color is added to what would otherwise be the material's visible color.
    /// This means that for a light emissive value, in darkness,
    /// you will mostly see the emissive component.
    ///
    /// The default emissive color is [`LinearRgba::BLACK`], which doesn't add anything to the material color.
    ///
    /// To increase emissive strength, channel values for `emissive`
    /// colors can exceed `1.0`. For instance, a `base_color` of
    /// `LinearRgba::rgb(1.0, 0.0, 0.0)` represents the brightest
    /// red for objects that reflect light, but an emissive color
    /// like `LinearRgba::rgb(1000.0, 0.0, 0.0)` can be used to create
    /// intensely bright red emissive effects.
    ///
    /// Increasing the emissive strength of the color will impact visual effects
    /// like bloom, but it's important to note that **an emissive material won't
    /// light up surrounding areas like a light source**,
    /// it just adds a value to the color seen on screen.
    pub emissive: LinearRgba,

    /// The weight in which the camera exposure influences the emissive color.
    /// A value of `0.0` means the emissive color is not affected by the camera exposure.
    /// In opposition, a value of `1.0` means the emissive color is multiplied by the camera exposure.
    ///
    /// Defaults to `0.0`
    pub emissive_exposure_weight: f32,

    /// The UV channel to use for the [`AnimatedArrayTextureMaterial::emissive_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub emissive_channel: UvChannel,

    /// The emissive map, multiplies pixels with [`emissive`]
    /// to get the final "emitting" color of a surface.
    ///
    /// This color is multiplied by [`emissive`] to get the final emitted color.
    /// Meaning that you should set [`emissive`] to [`Color::WHITE`]
    /// if you want to use the full range of color of the emissive texture.
    ///
    /// [`emissive`]: AnimatedArrayTextureMaterial::emissive
    #[texture(3)]
    #[sampler(4)]
    #[dependency]
    pub emissive_texture: Option<Handle<Image>>,

    /// Linear perceptual roughness, clamped to `[0.089, 1.0]` in the shader.
    ///
    /// Defaults to `0.5`.
    ///
    /// Low values result in a "glossy" material with specular highlights,
    /// while values close to `1` result in rough materials.
    ///
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `roughness * roughness_texture_value`.
    ///
    /// 0.089 is the minimum floating point value that won't be rounded down to 0 in the
    /// calculations used.
    //
    // Technically for 32-bit floats, 0.045 could be used.
    // See <https://google.github.io/filament/Filament.html#materialsystem/parameterization/>
    pub perceptual_roughness: f32,

    /// How "metallic" the material appears, within `[0.0, 1.0]`.
    ///
    /// This should be set to 0.0 for dielectric materials or 1.0 for metallic materials.
    /// For a hybrid surface such as corroded metal, you may need to use in-between values.
    ///
    /// Defaults to `0.00`, for dielectric.
    ///
    /// If used together with a roughness/metallic texture, this is factored into the final base
    /// color as `metallic * metallic_texture_value`.
    pub metallic: f32,

    /// The UV channel to use for the [`AnimatedArrayTextureMaterial::metallic_roughness_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub metallic_roughness_channel: UvChannel,

    /// Metallic and roughness maps, stored as a single texture.
    ///
    /// The blue channel contains metallic values,
    /// and the green channel contains the roughness values.
    /// Other channels are unused.
    ///
    /// Those values are multiplied by the scalar ones of the material,
    /// see [`metallic`] and [`perceptual_roughness`] for details.
    ///
    /// Note that with the default values of [`metallic`] and [`perceptual_roughness`],
    /// setting this texture has no effect. If you want to exclusively use the
    /// `metallic_roughness_texture` values for your material, make sure to set [`metallic`]
    /// and [`perceptual_roughness`] to `1.0`.
    ///
    /// [`metallic`]: AnimatedArrayTextureMaterial::metallic
    /// [`perceptual_roughness`]: AnimatedArrayTextureMaterial::perceptual_roughness
    #[texture(5)]
    #[sampler(6)]
    #[dependency]
    pub metallic_roughness_texture: Option<Handle<Image>>,

    /// Specular intensity for non-metals on a linear scale of `[0.0, 1.0]`.
    ///
    /// Use the value as a way to control the intensity of the
    /// specular highlight of the material, i.e. how reflective is the material,
    /// rather than the physical property "reflectance."
    ///
    /// Set to `0.0`, no specular highlight is visible, the highlight is strongest
    /// when `reflectance` is set to `1.0`.
    ///
    /// Defaults to `0.5` which is mapped to 4% reflectance in the shader.
    #[doc(alias = "specular_intensity")]
    pub reflectance: f32,

    /// The amount of light transmitted _diffusely_ through the material (i.e. “translucency”)
    ///
    /// Implemented as a second, flipped [Lambertian diffuse](https://en.wikipedia.org/wiki/Lambertian_reflectance) lobe,
    /// which provides an inexpensive but plausible approximation of translucency for thin dieletric objects (e.g. paper,
    /// leaves, some fabrics) or thicker volumetric materials with short scattering distances (e.g. porcelain, wax).
    ///
    /// For specular transmission usecases with refraction (e.g. glass) use the [`AnimatedArrayTextureMaterial::specular_transmission`] and
    /// [`AnimatedArrayTextureMaterial::ior`] properties instead.
    ///
    /// - When set to `0.0` (the default) no diffuse light is transmitted;
    /// - When set to `1.0` all diffuse light is transmitted through the material;
    /// - Values higher than `0.5` will cause more diffuse light to be transmitted than reflected, resulting in a “darker”
    ///   appearance on the side facing the light than the opposite side. (e.g. plant leaves)
    ///
    /// ## Notes
    ///
    /// - The material's [`AnimatedArrayTextureMaterial::base_color`] also modulates the transmitted light;
    /// - To receive transmitted shadows on the diffuse transmission lobe (i.e. the “backside”) of the material,
    ///   use the [`TransmittedShadowReceiver`] component.
    #[doc(alias = "translucency")]
    pub diffuse_transmission: f32,

    /// The amount of light transmitted _specularly_ through the material (i.e. via refraction)
    ///
    /// - When set to `0.0` (the default) no light is transmitted.
    /// - When set to `1.0` all light is transmitted through the material.
    ///
    /// The material's [`AnimatedArrayTextureMaterial::base_color`] also modulates the transmitted light.
    ///
    /// **Note:** Typically used in conjunction with [`AnimatedArrayTextureMaterial::thickness`], [`AnimatedArrayTextureMaterial::ior`] and [`AnimatedArrayTextureMaterial::perceptual_roughness`].
    ///
    /// ## Performance
    ///
    /// Specular transmission is implemented as a relatively expensive screen-space effect that allows ocluded objects to be seen through the material,
    /// with distortion and blur effects.
    ///
    /// - [`Camera3d::screen_space_specular_transmission_steps`](bevy_core_pipeline::core_3d::Camera3d::screen_space_specular_transmission_steps) can be used to enable transmissive objects
    ///   to be seen through other transmissive objects, at the cost of additional draw calls and texture copies; (Use with caution!)
    ///   - If a simplified approximation of specular transmission using only environment map lighting is sufficient, consider setting
    ///     [`Camera3d::screen_space_specular_transmission_steps`](bevy_core_pipeline::core_3d::Camera3d::screen_space_specular_transmission_steps) to `0`.
    /// - If purely diffuse light transmission is needed, (i.e. “translucency”) consider using [`AnimatedArrayTextureMaterial::diffuse_transmission`] instead,
    ///   for a much less expensive effect.
    /// - Specular transmission is rendered before alpha blending, so any material with [`AlphaMode::Blend`], [`AlphaMode::Premultiplied`], [`AlphaMode::Add`] or [`AlphaMode::Multiply`]
    ///   won't be visible through specular transmissive materials.
    #[doc(alias = "refraction")]
    pub specular_transmission: f32,

    /// Thickness of the volume beneath the material surface.
    ///
    /// When set to `0.0` (the default) the material appears as an infinitely-thin film,
    /// transmitting light without distorting it.
    ///
    /// When set to any other value, the material distorts light like a thick lens.
    ///
    /// **Note:** Typically used in conjunction with [`AnimatedArrayTextureMaterial::specular_transmission`] and [`AnimatedArrayTextureMaterial::ior`], or with
    /// [`AnimatedArrayTextureMaterial::diffuse_transmission`].
    #[doc(alias = "volume")]
    #[doc(alias = "thin_walled")]
    pub thickness: f32,

    /// The [index of refraction](https://en.wikipedia.org/wiki/Refractive_index) of the material.
    ///
    /// Defaults to 1.5.
    ///
    /// | Material        | Index of Refraction  |
    /// |:----------------|:---------------------|
    /// | Vacuum          | 1                    |
    /// | Air             | 1.00                 |
    /// | Ice             | 1.31                 |
    /// | Water           | 1.33                 |
    /// | Eyes            | 1.38                 |
    /// | Quartz          | 1.46                 |
    /// | Olive Oil       | 1.47                 |
    /// | Honey           | 1.49                 |
    /// | Acrylic         | 1.49                 |
    /// | Window Glass    | 1.52                 |
    /// | Polycarbonate   | 1.58                 |
    /// | Flint Glass     | 1.69                 |
    /// | Ruby            | 1.71                 |
    /// | Glycerine       | 1.74                 |
    /// | Sapphire        | 1.77                 |
    /// | Cubic Zirconia  | 2.15                 |
    /// | Diamond         | 2.42                 |
    /// | Moissanite      | 2.65                 |
    ///
    /// **Note:** Typically used in conjunction with [`AnimatedArrayTextureMaterial::specular_transmission`] and [`AnimatedArrayTextureMaterial::thickness`].
    #[doc(alias = "index_of_refraction")]
    #[doc(alias = "refraction_index")]
    #[doc(alias = "refractive_index")]
    pub ior: f32,

    /// How far, on average, light travels through the volume beneath the material's
    /// surface before being absorbed.
    ///
    /// Defaults to [`f32::INFINITY`], i.e. light is never absorbed.
    ///
    /// **Note:** To have any effect, must be used in conjunction with:
    /// - [`AnimatedArrayTextureMaterial::attenuation_color`];
    /// - [`AnimatedArrayTextureMaterial::thickness`];
    /// - [`AnimatedArrayTextureMaterial::diffuse_transmission`] or [`AnimatedArrayTextureMaterial::specular_transmission`].
    #[doc(alias = "absorption_distance")]
    #[doc(alias = "extinction_distance")]
    pub attenuation_distance: f32,

    /// The resulting (non-absorbed) color after white light travels through the attenuation distance.
    ///
    /// Defaults to [`Color::WHITE`], i.e. no change.
    ///
    /// **Note:** To have any effect, must be used in conjunction with:
    /// - [`AnimatedArrayTextureMaterial::attenuation_distance`];
    /// - [`AnimatedArrayTextureMaterial::thickness`];
    /// - [`AnimatedArrayTextureMaterial::diffuse_transmission`] or [`AnimatedArrayTextureMaterial::specular_transmission`].
    #[doc(alias = "absorption_color")]
    #[doc(alias = "extinction_color")]
    pub attenuation_color: Color,

    /// The UV channel to use for the [`AnimatedArrayTextureMaterial::normal_map_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub normal_map_channel: UvChannel,

    /// Used to fake the lighting of bumps and dents on a material.
    ///
    /// A typical usage would be faking cobblestones on a flat plane mesh in 3D.
    ///
    /// # Notes
    ///
    /// Normal mapping with `AnimatedArrayTextureMaterial` and the core bevy PBR shaders requires:
    /// - A normal map texture
    /// - Vertex UVs
    /// - Vertex tangents
    /// - Vertex normals
    ///
    /// Tangents do not have to be stored in your model,
    /// they can be generated using the [`Mesh::generate_tangents`] or
    /// [`Mesh::with_generated_tangents`] methods.
    /// If your material has a normal map, but still renders as a flat surface,
    /// make sure your meshes have their tangents set.
    ///
    /// [`Mesh::generate_tangents`]: bevy_render::mesh::Mesh::generate_tangents
    /// [`Mesh::with_generated_tangents`]: bevy_render::mesh::Mesh::with_generated_tangents
    #[texture(9)]
    #[sampler(10)]
    #[dependency]
    pub normal_map_texture: Option<Handle<Image>>,

    /// Normal map textures authored for DirectX have their y-component flipped. Set this to flip
    /// it to right-handed conventions.
    pub flip_normal_map_y: bool,

    /// The UV channel to use for the [`AnimatedArrayTextureMaterial::occlusion_texture`].
    ///
    /// Defaults to [`UvChannel::Uv0`].
    pub occlusion_channel: UvChannel,

    /// Specifies the level of exposure to ambient light.
    ///
    /// This is usually generated and stored automatically ("baked") by 3D-modelling software.
    ///
    /// Typically, steep concave parts of a model (such as the armpit of a shirt) are darker,
    /// because they have little exposure to light.
    /// An occlusion map specifies those parts of the model that light doesn't reach well.
    ///
    /// The material will be less lit in places where this texture is dark.
    /// This is similar to ambient occlusion, but built into the model.
    #[texture(7)]
    #[sampler(8)]
    #[dependency]
    pub occlusion_texture: Option<Handle<Image>>,

    /// An extra thin translucent layer on top of the main PBR layer. This is
    /// typically used for painted surfaces.
    ///
    /// This value specifies the strength of the layer, which affects how
    /// visible the clearcoat layer will be.
    ///
    /// Defaults to zero, specifying no clearcoat layer.
    pub clearcoat: f32,

    /// The roughness of the clearcoat material. This is specified in exactly
    /// the same way as the [`AnimatedArrayTextureMaterial::perceptual_roughness`].
    ///
    /// If the [`AnimatedArrayTextureMaterial::clearcoat`] value if zero, this has no
    /// effect.
    ///
    /// Defaults to 0.5.
    pub clearcoat_perceptual_roughness: f32,

    /// Increases the roughness along a specific direction, so that the specular
    /// highlight will be stretched instead of being a circular lobe.
    ///
    /// This value ranges from 0 (perfectly circular) to 1 (maximally
    /// stretched). The default direction (corresponding to a
    /// [`AnimatedArrayTextureMaterial::anisotropy_rotation`] of 0) aligns with the
    /// *tangent* of the mesh; thus mesh tangents must be specified in order for
    /// this parameter to have any meaning. The direction can be changed using
    /// the [`AnimatedArrayTextureMaterial::anisotropy_rotation`] parameter.
    ///
    /// This is typically used for modeling surfaces such as brushed metal and
    /// hair, in which one direction of the surface but not the other is smooth.
    ///
    /// See the [`KHR_materials_anisotropy` specification] for more details.
    ///
    /// [`KHR_materials_anisotropy` specification]:
    /// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md
    pub anisotropy_strength: f32,

    /// The direction of increased roughness, in radians relative to the mesh
    /// tangent.
    ///
    /// This parameter causes the roughness to vary according to the
    /// [`AnimatedArrayTextureMaterial::anisotropy_strength`]. The rotation is applied in
    /// tangent-bitangent space; thus, mesh tangents must be present for this
    /// parameter to have any meaning.
    ///
    /// This parameter has no effect if
    /// [`AnimatedArrayTextureMaterial::anisotropy_strength`] is zero. Its value can
    /// optionally be adjusted across the mesh with the
    /// [`AnimatedArrayTextureMaterial::anisotropy_texture`].
    ///
    /// See the [`KHR_materials_anisotropy` specification] for more details.
    ///
    /// [`KHR_materials_anisotropy` specification]:
    /// https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md
    pub anisotropy_rotation: f32,

    /// Support two-sided lighting by automatically flipping the normals for "back" faces
    /// within the PBR lighting shader.
    ///
    /// Defaults to `false`.
    /// This does not automatically configure backface culling,
    /// which can be done via `cull_mode`.
    pub double_sided: bool,

    /// Whether to cull the "front", "back" or neither side of a mesh.
    /// If set to `None`, the two sides of the mesh are visible.
    ///
    /// Defaults to `Some(Face::Back)`.
    /// In bevy, the order of declaration of a triangle's vertices
    /// in [`Mesh`] defines the triangle's front face.
    ///
    /// When a triangle is in a viewport,
    /// if its vertices appear counter-clockwise from the viewport's perspective,
    /// then the viewport is seeing the triangle's front face.
    /// Conversely, if the vertices appear clockwise, you are seeing the back face.
    ///
    /// In short, in bevy, front faces winds counter-clockwise.
    ///
    /// Your 3D editing software should manage all of that.
    ///
    /// [`Mesh`]: bevy_render::mesh::Mesh
    // TODO: include this in reflection somehow (maybe via remote types like serde https://serde.rs/remote-derive.html)
    #[reflect(ignore)]
    pub cull_mode: Option<Face>,

    /// Whether to apply only the base color to this material.
    ///
    /// Normals, occlusion textures, roughness, metallic, reflectance, emissive,
    /// shadows, alpha mode and ambient light are ignored if this is set to `true`.
    pub unlit: bool,

    /// Whether to enable fog for this material.
    pub fog_enabled: bool,

    /// How to apply the alpha channel of the `base_color_texture`.
    ///
    /// See [`AlphaMode`] for details. Defaults to [`AlphaMode::Opaque`].
    pub alpha_mode: AlphaMode,

    /// Adjust rendered depth.
    ///
    /// A material with a positive depth bias will render closer to the
    /// camera while negative values cause the material to render behind
    /// other objects. This is independent of the viewport.
    ///
    /// `depth_bias` affects render ordering and depth write operations
    /// using the `wgpu::DepthBiasState::Constant` field.
    ///
    /// [z-fighting]: https://en.wikipedia.org/wiki/Z-fighting
    pub depth_bias: f32,

    /// The depth map used for [parallax mapping].
    ///
    /// It is a greyscale image where white represents bottom and black the top.
    /// If this field is set, bevy will apply [parallax mapping].
    /// Parallax mapping, unlike simple normal maps, will move the texture
    /// coordinate according to the current perspective,
    /// giving actual depth to the texture.
    ///
    /// The visual result is similar to a displacement map,
    /// but does not require additional geometry.
    ///
    /// Use the [`parallax_depth_scale`] field to control the depth of the parallax.
    ///
    /// ## Limitations
    ///
    /// - It will look weird on bent/non-planar surfaces.
    /// - The depth of the pixel does not reflect its visual position, resulting
    ///   in artifacts for depth-dependent features such as fog or SSAO.
    /// - For the same reason, the geometry silhouette will always be
    ///   the one of the actual geometry, not the parallaxed version, resulting
    ///   in awkward looks on intersecting parallaxed surfaces.
    ///
    /// ## Performance
    ///
    /// Parallax mapping requires multiple texture lookups, proportional to
    /// [`max_parallax_layer_count`], which might be costly.
    ///
    /// Use the [`parallax_mapping_method`] and [`max_parallax_layer_count`] fields
    /// to tweak the shader, trading graphical quality for performance.
    ///
    /// To improve performance, set your `depth_map`'s [`Image::sampler`]
    /// filter mode to `FilterMode::Nearest`, as [this paper] indicates, it improves
    /// performance a bit.
    ///
    /// To reduce artifacts, avoid steep changes in depth, blurring the depth
    /// map helps with this.
    ///
    /// Larger depth maps haves a disproportionate performance impact.
    ///
    /// [this paper]: https://www.diva-portal.org/smash/get/diva2:831762/FULLTEXT01.pdf
    /// [parallax mapping]: https://en.wikipedia.org/wiki/Parallax_mapping
    /// [`parallax_depth_scale`]: AnimatedArrayTextureMaterial::parallax_depth_scale
    /// [`parallax_mapping_method`]: AnimatedArrayTextureMaterial::parallax_mapping_method
    /// [`max_parallax_layer_count`]: AnimatedArrayTextureMaterial::max_parallax_layer_count
    #[texture(11)]
    #[sampler(12)]
    #[dependency]
    pub depth_map: Option<Handle<Image>>,

    /// How deep the offset introduced by the depth map should be.
    ///
    /// Default is `0.1`, anything over that value may look distorted.
    /// Lower values lessen the effect.
    ///
    /// The depth is relative to texture size. This means that if your texture
    /// occupies a surface of `1` world unit, and `parallax_depth_scale` is `0.1`, then
    /// the in-world depth will be of `0.1` world units.
    /// If the texture stretches for `10` world units, then the final depth
    /// will be of `1` world unit.
    pub parallax_depth_scale: f32,

    /// Which parallax mapping method to use.
    ///
    /// We recommend that all objects use the same [`ParallaxMappingMethod`], to avoid
    /// duplicating and running two shaders.
    pub parallax_mapping_method: ParallaxMappingMethod,

    /// In how many layers to split the depth maps for parallax mapping.
    ///
    /// If you are seeing jaggy edges, increase this value.
    /// However, this incurs a performance cost.
    ///
    /// Dependent on the situation, switching to [`ParallaxMappingMethod::Relief`]
    /// and keeping this value low might have better performance than increasing the
    /// layer count while using [`ParallaxMappingMethod::Occlusion`].
    ///
    /// Default is `16.0`.
    pub max_parallax_layer_count: f32,

    /// The exposure (brightness) level of the lightmap, if present.
    pub lightmap_exposure: f32,

    /// Render method used for opaque materials. (Where `alpha_mode` is [`AlphaMode::Opaque`] or [`AlphaMode::Mask`])
    pub opaque_render_method: OpaqueRendererMethod,

    /// Used for selecting the deferred lighting pass for deferred materials.
    /// Default is [`DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID`] for default
    /// PBR deferred lighting pass. Ignored in the case of forward materials.
    pub deferred_lighting_pass_id: u8,

    /// The transform applied to the UVs corresponding to `ATTRIBUTE_UV_0` on the mesh before sampling. Default is identity.
    pub uv_transform: Affine2,
}

impl AnimatedArrayTextureMaterial {
    /// Horizontal flipping transform
    ///
    /// Multiplying this with another Affine2 returns transformation with horizontally flipped texture coords
    pub const FLIP_HORIZONTAL: Affine2 = Affine2 {
        matrix2: Mat2::from_cols(Vec2::new(-1.0, 0.0), Vec2::Y),
        translation: Vec2::X,
    };

    /// Vertical flipping transform
    ///
    /// Multiplying this with another Affine2 returns transformation with vertically flipped texture coords
    pub const FLIP_VERTICAL: Affine2 = Affine2 {
        matrix2: Mat2::from_cols(Vec2::X, Vec2::new(0.0, -1.0)),
        translation: Vec2::Y,
    };

    /// Flipping X 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped X coords
    pub const FLIP_X: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::new(-1.0, 0.0, 0.0), Vec3::Y, Vec3::Z),
        translation: Vec3::X,
    };

    /// Flipping Y 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped Y coords
    pub const FLIP_Y: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::X, Vec3::new(0.0, -1.0, 0.0), Vec3::Z),
        translation: Vec3::Y,
    };

    /// Flipping Z 3D transform
    ///
    /// Multiplying this with another Affine3 returns transformation with flipped Z coords
    pub const FLIP_Z: Affine3 = Affine3 {
        matrix3: Mat3::from_cols(Vec3::X, Vec3::Y, Vec3::new(0.0, 0.0, -1.0)),
        translation: Vec3::Z,
    };

    /// Flip the texture coordinates of the material.
    pub fn flip(&mut self, horizontal: bool, vertical: bool) {
        if horizontal {
            // Multiplication of `Affine2` is order dependent, which is why
            // we do not use the `*=` operator.
            self.uv_transform = Self::FLIP_HORIZONTAL * self.uv_transform;
        }
        if vertical {
            self.uv_transform = Self::FLIP_VERTICAL * self.uv_transform;
        }
    }

    /// Consumes the material and returns a material with flipped texture coordinates
    pub fn flipped(mut self, horizontal: bool, vertical: bool) -> Self {
        self.flip(horizontal, vertical);
        self
    }

    /// Creates a new material from a given color
    pub fn from_color(color: impl Into<Color>) -> Self {
        Self::from(color.into())
    }
}

impl Default for AnimatedArrayTextureMaterial {
    fn default() -> Self {
        AnimatedArrayTextureMaterial {
            // White because it gets multiplied with texture values if someone uses
            // a texture.
            base_color: Color::WHITE,
            base_color_channel: UvChannel::Uv0,
            base_color_texture: None,
            emissive: LinearRgba::BLACK,
            emissive_exposure_weight: 0.0,
            emissive_channel: UvChannel::Uv0,
            emissive_texture: None,
            // Matches Blender's default roughness.
            perceptual_roughness: 0.5,
            // Metallic should generally be set to 0.0 or 1.0.
            metallic: 0.0,
            metallic_roughness_channel: UvChannel::Uv0,
            metallic_roughness_texture: None,
            // Minimum real-world reflectance is 2%, most materials between 2-5%
            // Expressed in a linear scale and equivalent to 4% reflectance see
            // <https://google.github.io/filament/Material%20Properties.pdf>
            reflectance: 0.5,
            diffuse_transmission: 0.0,
            specular_transmission: 0.0,
            thickness: 0.0,
            ior: 1.5,
            attenuation_color: Color::WHITE,
            attenuation_distance: f32::INFINITY,
            occlusion_channel: UvChannel::Uv0,
            occlusion_texture: None,
            normal_map_channel: UvChannel::Uv0,
            normal_map_texture: None,
            clearcoat: 0.0,
            clearcoat_perceptual_roughness: 0.5,
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            flip_normal_map_y: false,
            double_sided: false,
            cull_mode: Some(Face::Back),
            unlit: false,
            fog_enabled: true,
            alpha_mode: AlphaMode::Opaque,
            depth_bias: 0.0,
            depth_map: None,
            parallax_depth_scale: 0.1,
            max_parallax_layer_count: 16.0,
            lightmap_exposure: 1.0,
            parallax_mapping_method: ParallaxMappingMethod::Occlusion,
            opaque_render_method: OpaqueRendererMethod::Auto,
            deferred_lighting_pass_id: DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID,
            uv_transform: Affine2::IDENTITY,
        }
    }
}

impl From<Color> for AnimatedArrayTextureMaterial {
    fn from(color: Color) -> Self {
        AnimatedArrayTextureMaterial {
            base_color: color,
            alpha_mode: if color.alpha() < 1.0 { AlphaMode::Blend } else { AlphaMode::Opaque },
            ..Default::default()
        }
    }
}

impl From<Handle<Image>> for AnimatedArrayTextureMaterial {
    fn from(texture: Handle<Image>) -> Self {
        AnimatedArrayTextureMaterial {
            base_color_texture: Some(texture),
            ..Default::default()
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/pbr_types.wgsl!
bitflags::bitflags! {
    /// Bitflags info about the material a shader is currently rendering.
    /// This is accessible in the shader in the [`AnimatedArrayTextureMaterialUniform`]
    #[repr(transparent)]
    pub struct AnimatedArrayTextureMaterialFlags: u32 {
        /// From [`bevy::prelude::StandardMaterial`]
        const BASE_COLOR_TEXTURE         = 1 << 0;
        /// From [`bevy::prelude::StandardMaterial`]
        const EMISSIVE_TEXTURE           = 1 << 1;
        /// From [`bevy::prelude::StandardMaterial`]
        const METALLIC_ROUGHNESS_TEXTURE = 1 << 2;
        /// From [`bevy::prelude::StandardMaterial`]
        const OCCLUSION_TEXTURE          = 1 << 3;
        /// From [`bevy::prelude::StandardMaterial`]
        const DOUBLE_SIDED               = 1 << 4;
        /// From [`bevy::prelude::StandardMaterial`]
        const UNLIT                      = 1 << 5;
        /// From [`bevy::prelude::StandardMaterial`]
        const TWO_COMPONENT_NORMAL_MAP   = 1 << 6;
        /// From [`bevy::prelude::StandardMaterial`]
        const FLIP_NORMAL_MAP_Y          = 1 << 7;
        /// From [`bevy::prelude::StandardMaterial`]
        const FOG_ENABLED                = 1 << 8;
        /// From [`bevy::prelude::StandardMaterial`]
        const DEPTH_MAP                  = 1 << 9; // Used for parallax mapping
        /// From [`bevy::prelude::StandardMaterial`]
        const SPECULAR_TRANSMISSION_TEXTURE = 1 << 10;
        /// From [`bevy::prelude::StandardMaterial`]
        const THICKNESS_TEXTURE          = 1 << 11;
        /// From [`bevy::prelude::StandardMaterial`]
        const DIFFUSE_TRANSMISSION_TEXTURE = 1 << 12;
        /// From [`bevy::prelude::StandardMaterial`]
        const ATTENUATION_ENABLED        = 1 << 13;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_TEXTURE          = 1 << 14;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_ROUGHNESS_TEXTURE = 1 << 15;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_NORMAL_TEXTURE   = 1 << 16;
        /// From [`bevy::prelude::StandardMaterial`]
        const ANISOTROPY_TEXTURE         = 1 << 17;
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_RESERVED_BITS   = Self::ALPHA_MODE_MASK_BITS << Self::ALPHA_MODE_SHIFT_BITS; // ← Bitmask reserving bits for the `AlphaMode`
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_OPAQUE          = 0 << Self::ALPHA_MODE_SHIFT_BITS;                          // ← Values are just sequential values bitshifted into
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_MASK            = 1 << Self::ALPHA_MODE_SHIFT_BITS;                          //   the bitmask, and can range from 0 to 7.
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_BLEND           = 2 << Self::ALPHA_MODE_SHIFT_BITS;                          //
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_PREMULTIPLIED   = 3 << Self::ALPHA_MODE_SHIFT_BITS;                          //
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_ADD             = 4 << Self::ALPHA_MODE_SHIFT_BITS;                          //   Right now only values 0–5 are used, which still gives
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_MULTIPLY        = 5 << Self::ALPHA_MODE_SHIFT_BITS;                          // ← us "room" for two more modes without adding more bits
        /// From [`bevy::prelude::StandardMaterial`]
        const ALPHA_MODE_ALPHA_TO_COVERAGE = 6 << Self::ALPHA_MODE_SHIFT_BITS;
        /// From [`bevy::prelude::StandardMaterial`]
        const NONE                       = 0;
        /// From [`bevy::prelude::StandardMaterial`]
        const UNINITIALIZED              = 0xFFFF;
    }
}

impl AnimatedArrayTextureMaterialFlags {
    const ALPHA_MODE_MASK_BITS: u32 = 0b111;
    const ALPHA_MODE_SHIFT_BITS: u32 = 32 - Self::ALPHA_MODE_MASK_BITS.count_ones();
}

/// The GPU representation of the uniform data of a [`AnimatedArrayTextureMaterial`].
#[derive(Clone, Default, ShaderType)]
pub struct AnimatedArrayTextureMaterialUniform {
    /// Doubles as diffuse albedo for non-metallic, specular for metallic and a mix for everything
    /// in between.
    pub base_color: Vec4,
    /// Use a color for user-friendliness even though we technically don't use the alpha channel
    /// Might be used in the future for exposure correction in HDR
    pub emissive: Vec4,
    /// Color white light takes after travelling through the attenuation distance underneath the material surface
    pub attenuation_color: Vec4,
    /// The transform applied to the UVs corresponding to `ATTRIBUTE_UV_0` on the mesh before sampling. Default is identity.
    pub uv_transform: Mat3,
    /// Linear perceptual roughness, clamped to [0.089, 1.0] in the shader
    /// Defaults to minimum of 0.089
    pub roughness: f32,
    /// From [0.0, 1.0], dielectric to pure metallic
    pub metallic: f32,
    /// Specular intensity for non-metals on a linear scale of [0.0, 1.0]
    /// defaults to 0.5 which is mapped to 4% reflectance in the shader
    pub reflectance: f32,
    /// Amount of diffuse light transmitted through the material
    pub diffuse_transmission: f32,
    /// Amount of specular light transmitted through the material
    pub specular_transmission: f32,
    /// Thickness of the volume underneath the material surface
    pub thickness: f32,
    /// Index of Refraction
    pub ior: f32,
    /// How far light travels through the volume underneath the material surface before being absorbed
    pub attenuation_distance: f32,
    /// From [`bevy::prelude::StandardMaterial`]
    pub clearcoat: f32,
    /// From [`bevy::prelude::StandardMaterial`]
    pub clearcoat_perceptual_roughness: f32,
    /// From [`bevy::prelude::StandardMaterial`]
    pub anisotropy_strength: f32,
    /// From [`bevy::prelude::StandardMaterial`]
    pub anisotropy_rotation: Vec2,
    /// The [`AnimatedArrayTextureMaterialFlags`] accessible in the `wgsl` shader.
    pub flags: u32,
    /// When the alpha mode mask flag is set, any base color alpha above this cutoff means fully opaque,
    /// and any below means fully transparent.
    pub alpha_cutoff: f32,
    /// The depth of the [`AnimatedArrayTextureMaterial::depth_map`] to apply.
    pub parallax_depth_scale: f32,
    /// In how many layers to split the depth maps for Steep parallax mapping.
    ///
    /// If your `parallax_depth_scale` is >0.1 and you are seeing jaggy edges,
    /// increase this value. However, this incurs a performance cost.
    pub max_parallax_layer_count: f32,
    /// The exposure (brightness) level of the lightmap, if present.
    pub lightmap_exposure: f32,
    /// Using [`ParallaxMappingMethod::Relief`], how many additional
    /// steps to use at most to find the depth value.
    pub max_relief_mapping_search_steps: u32,
    /// ID for specifying which deferred lighting pass should be used for rendering this material, if any.
    pub deferred_lighting_pass_id: u32,
}

impl AsBindGroupShaderType<AnimatedArrayTextureMaterialUniform> for AnimatedArrayTextureMaterial {
    fn as_bind_group_shader_type(&self, images: &RenderAssets<GpuImage>) -> AnimatedArrayTextureMaterialUniform {
        let mut flags = AnimatedArrayTextureMaterialFlags::NONE;
        if self.base_color_texture.is_some() {
            flags |= AnimatedArrayTextureMaterialFlags::BASE_COLOR_TEXTURE;
        }
        if self.emissive_texture.is_some() {
            flags |= AnimatedArrayTextureMaterialFlags::EMISSIVE_TEXTURE;
        }
        if self.metallic_roughness_texture.is_some() {
            flags |= AnimatedArrayTextureMaterialFlags::METALLIC_ROUGHNESS_TEXTURE;
        }
        if self.occlusion_texture.is_some() {
            flags |= AnimatedArrayTextureMaterialFlags::OCCLUSION_TEXTURE;
        }
        if self.double_sided {
            flags |= AnimatedArrayTextureMaterialFlags::DOUBLE_SIDED;
        }
        if self.unlit {
            flags |= AnimatedArrayTextureMaterialFlags::UNLIT;
        }
        if self.fog_enabled {
            flags |= AnimatedArrayTextureMaterialFlags::FOG_ENABLED;
        }
        if self.depth_map.is_some() {
            flags |= AnimatedArrayTextureMaterialFlags::DEPTH_MAP;
        }

        let has_normal_map = self.normal_map_texture.is_some();
        if has_normal_map {
            let normal_map_id = self.normal_map_texture.as_ref().map(Handle::id).unwrap();
            if let Some(texture) = images.get(normal_map_id) {
                match texture.texture_format {
                    // All 2-component unorm formats
                    TextureFormat::Rg8Unorm | TextureFormat::Rg16Unorm | TextureFormat::Bc5RgUnorm | TextureFormat::EacRg11Unorm => {
                        flags |= AnimatedArrayTextureMaterialFlags::TWO_COMPONENT_NORMAL_MAP;
                    }
                    _ => {}
                }
            }
            if self.flip_normal_map_y {
                flags |= AnimatedArrayTextureMaterialFlags::FLIP_NORMAL_MAP_Y;
            }
        }
        // NOTE: 0.5 is from the glTF default - do we want this?
        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode::Opaque => flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode::Blend => flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_BLEND,
            AlphaMode::Premultiplied => flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_PREMULTIPLIED,
            AlphaMode::Add => flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_ADD,
            AlphaMode::Multiply => flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_MULTIPLY,
            AlphaMode::AlphaToCoverage => {
                flags |= AnimatedArrayTextureMaterialFlags::ALPHA_MODE_ALPHA_TO_COVERAGE;
            }
        };

        if self.attenuation_distance.is_finite() {
            flags |= AnimatedArrayTextureMaterialFlags::ATTENUATION_ENABLED;
        }

        let mut emissive = self.emissive.to_vec4();
        emissive[3] = self.emissive_exposure_weight;

        // Doing this up front saves having to do this repeatedly in the fragment shader.
        let anisotropy_rotation = vec2(self.anisotropy_rotation.cos(), self.anisotropy_rotation.sin());

        AnimatedArrayTextureMaterialUniform {
            base_color: LinearRgba::from(self.base_color).to_vec4(),
            emissive,
            roughness: self.perceptual_roughness,
            metallic: self.metallic,
            reflectance: self.reflectance,
            clearcoat: self.clearcoat,
            clearcoat_perceptual_roughness: self.clearcoat_perceptual_roughness,
            anisotropy_strength: self.anisotropy_strength,
            anisotropy_rotation,
            diffuse_transmission: self.diffuse_transmission,
            specular_transmission: self.specular_transmission,
            thickness: self.thickness,
            ior: self.ior,
            attenuation_distance: self.attenuation_distance,
            attenuation_color: LinearRgba::from(self.attenuation_color).to_f32_array().into(),
            flags: flags.bits(),
            alpha_cutoff,
            parallax_depth_scale: self.parallax_depth_scale,
            max_parallax_layer_count: self.max_parallax_layer_count,
            lightmap_exposure: self.lightmap_exposure,
            max_relief_mapping_search_steps: match self.parallax_mapping_method {
                ParallaxMappingMethod::Occlusion => 0,
                ParallaxMappingMethod::Relief { max_steps } => max_steps,
            },
            deferred_lighting_pass_id: self.deferred_lighting_pass_id as u32,
            uv_transform: self.uv_transform.into(),
        }
    }
}

bitflags! {
    /// The pipeline key for `AnimatedArrayTextureMaterial`, packed into 64 bits.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct BlockMaterialKey: u64 {
        /// From [`bevy::prelude::StandardMaterial`]
        const CULL_FRONT               = 0x000001;
        /// From [`bevy::prelude::StandardMaterial`]
        const CULL_BACK                = 0x000002;
        /// From [`bevy::prelude::StandardMaterial`]
        const NORMAL_MAP               = 0x000004;
        /// From [`bevy::prelude::StandardMaterial`]
        const RELIEF_MAPPING           = 0x000008;
        /// From [`bevy::prelude::StandardMaterial`]
        const DIFFUSE_TRANSMISSION     = 0x000010;
        /// From [`bevy::prelude::StandardMaterial`]
        const SPECULAR_TRANSMISSION    = 0x000020;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT                = 0x000040;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_NORMAL_MAP     = 0x000080;
        /// From [`bevy::prelude::StandardMaterial`]
        const ANISOTROPY               = 0x000100;
        /// From [`bevy::prelude::StandardMaterial`]
        const BASE_COLOR_UV            = 0x000200;
        /// From [`bevy::prelude::StandardMaterial`]
        const EMISSIVE_UV              = 0x000400;
        /// From [`bevy::prelude::StandardMaterial`]
        const METALLIC_ROUGHNESS_UV    = 0x000800;
        /// From [`bevy::prelude::StandardMaterial`]
        const OCCLUSION_UV             = 0x001000;
        /// From [`bevy::prelude::StandardMaterial`]
        const SPECULAR_TRANSMISSION_UV = 0x002000;
        /// From [`bevy::prelude::StandardMaterial`]
        const THICKNESS_UV             = 0x004000;
        /// From [`bevy::prelude::StandardMaterial`]
        const DIFFUSE_TRANSMISSION_UV  = 0x008000;
        /// From [`bevy::prelude::StandardMaterial`]
        const NORMAL_MAP_UV            = 0x010000;
        /// From [`bevy::prelude::StandardMaterial`]
        const ANISOTROPY_UV            = 0x020000;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_UV             = 0x040000;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_ROUGHNESS_UV   = 0x080000;
        /// From [`bevy::prelude::StandardMaterial`]
        const CLEARCOAT_NORMAL_UV      = 0x100000;
        /// From [`bevy::prelude::StandardMaterial`]
        const DEPTH_BIAS               = 0xffffffff_00000000;
    }
}

const STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT: u64 = 32;

impl From<&AnimatedArrayTextureMaterial> for BlockMaterialKey {
    fn from(material: &AnimatedArrayTextureMaterial) -> Self {
        let mut key = BlockMaterialKey::empty();
        key.set(BlockMaterialKey::CULL_FRONT, material.cull_mode == Some(Face::Front));
        key.set(BlockMaterialKey::CULL_BACK, material.cull_mode == Some(Face::Back));
        key.set(BlockMaterialKey::NORMAL_MAP, material.normal_map_texture.is_some());
        key.set(
            BlockMaterialKey::RELIEF_MAPPING,
            matches!(material.parallax_mapping_method, ParallaxMappingMethod::Relief { .. }),
        );
        key.set(BlockMaterialKey::DIFFUSE_TRANSMISSION, material.diffuse_transmission > 0.0);
        key.set(BlockMaterialKey::SPECULAR_TRANSMISSION, material.specular_transmission > 0.0);

        key.set(BlockMaterialKey::CLEARCOAT, material.clearcoat > 0.0);

        key.set(BlockMaterialKey::ANISOTROPY, material.anisotropy_strength > 0.0);

        key.set(BlockMaterialKey::BASE_COLOR_UV, material.base_color_channel != UvChannel::Uv0);

        key.set(BlockMaterialKey::EMISSIVE_UV, material.emissive_channel != UvChannel::Uv0);
        key.set(
            BlockMaterialKey::METALLIC_ROUGHNESS_UV,
            material.metallic_roughness_channel != UvChannel::Uv0,
        );
        key.set(BlockMaterialKey::OCCLUSION_UV, material.occlusion_channel != UvChannel::Uv0);

        key.set(BlockMaterialKey::NORMAL_MAP_UV, material.normal_map_channel != UvChannel::Uv0);

        key.insert(BlockMaterialKey::from_bits_retain(
            (material.depth_bias as u64) << STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT,
        ));
        key
    }
}

impl Material for AnimatedArrayTextureMaterial {
    fn prepass_vertex_shader() -> ShaderRef {
        "cosmos/shaders/animated_prepass.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "cosmos/shaders/animated_prepass.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "cosmos/shaders/animated.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "cosmos/shaders/animated.wgsl".into()
    }

    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    #[inline]
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        match self.opaque_render_method {
            // For now, diffuse transmission doesn't work under deferred rendering as we don't pack
            // the required data into the GBuffer. If this material is set to `Auto`, we report it as
            // `Forward` so that it's rendered correctly, even when the `DefaultOpaqueRendererMethod`
            // is set to `Deferred`.
            //
            // If the developer explicitly sets the `OpaqueRendererMethod` to `Deferred`, we assume
            // they know what they're doing and don't override it.
            OpaqueRendererMethod::Auto if self.diffuse_transmission > 0.0 => OpaqueRendererMethod::Forward,
            other => other,
        }
    }

    #[inline]
    fn depth_bias(&self) -> f32 {
        self.depth_bias
    }

    #[inline]
    fn reads_view_transmission_texture(&self) -> bool {
        self.specular_transmission > 0.0
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(fragment) = descriptor.fragment.as_mut() {
            let shader_defs = &mut fragment.shader_defs;

            for (flags, shader_def) in [
                (BlockMaterialKey::NORMAL_MAP, "STANDARD_MATERIAL_NORMAL_MAP"),
                (BlockMaterialKey::RELIEF_MAPPING, "RELIEF_MAPPING"),
                (BlockMaterialKey::DIFFUSE_TRANSMISSION, "STANDARD_MATERIAL_DIFFUSE_TRANSMISSION"),
                (BlockMaterialKey::SPECULAR_TRANSMISSION, "STANDARD_MATERIAL_SPECULAR_TRANSMISSION"),
                (
                    BlockMaterialKey::DIFFUSE_TRANSMISSION | BlockMaterialKey::SPECULAR_TRANSMISSION,
                    "STANDARD_MATERIAL_DIFFUSE_OR_SPECULAR_TRANSMISSION",
                ),
                (BlockMaterialKey::CLEARCOAT, "STANDARD_MATERIAL_CLEARCOAT"),
                (BlockMaterialKey::CLEARCOAT_NORMAL_MAP, "STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP"),
                (BlockMaterialKey::ANISOTROPY, "STANDARD_MATERIAL_ANISOTROPY"),
                (BlockMaterialKey::BASE_COLOR_UV, "STANDARD_MATERIAL_BASE_COLOR_UV_B"),
                (BlockMaterialKey::EMISSIVE_UV, "STANDARD_MATERIAL_EMISSIVE_UV_B"),
                (BlockMaterialKey::METALLIC_ROUGHNESS_UV, "STANDARD_MATERIAL_METALLIC_ROUGHNESS_UV_B"),
                (BlockMaterialKey::OCCLUSION_UV, "STANDARD_MATERIAL_OCCLUSION_UV_B"),
                (
                    BlockMaterialKey::SPECULAR_TRANSMISSION_UV,
                    "STANDARD_MATERIAL_SPECULAR_TRANSMISSION_UV_B",
                ),
                (BlockMaterialKey::THICKNESS_UV, "STANDARD_MATERIAL_THICKNESS_UV_B"),
                (
                    BlockMaterialKey::DIFFUSE_TRANSMISSION_UV,
                    "STANDARD_MATERIAL_DIFFUSE_TRANSMISSION_UV_B",
                ),
                (BlockMaterialKey::NORMAL_MAP_UV, "STANDARD_MATERIAL_NORMAL_MAP_UV_B"),
                (BlockMaterialKey::CLEARCOAT_UV, "STANDARD_MATERIAL_CLEARCOAT_UV_B"),
                (
                    BlockMaterialKey::CLEARCOAT_ROUGHNESS_UV,
                    "STANDARD_MATERIAL_CLEARCOAT_ROUGHNESS_UV_B",
                ),
                (BlockMaterialKey::CLEARCOAT_NORMAL_UV, "STANDARD_MATERIAL_CLEARCOAT_NORMAL_UV_B"),
                (BlockMaterialKey::ANISOTROPY_UV, "STANDARD_MATERIAL_ANISOTROPY_UV"),
            ] {
                if key.bind_group_data.intersects(flags) {
                    shader_defs.push(shader_def.into());
                }
            }
        }

        descriptor.primitive.cull_mode = if key.bind_group_data.contains(BlockMaterialKey::CULL_FRONT) {
            Some(Face::Front)
        } else if key.bind_group_data.contains(BlockMaterialKey::CULL_BACK) {
            Some(Face::Back)
        } else {
            None
        };

        if let Some(label) = &mut descriptor.label {
            *label = format!("pbr_{}", *label).into();
        }
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.bias.constant = (key.bind_group_data.bits() >> STANDARD_MATERIAL_KEY_DEPTH_BIAS_SHIFT) as i32;
        }

        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_TEXTURE_INDEX.at_shader_location(20),
            ATTRIBUTE_PACKED_ANIMATION_DATA.at_shader_location(21),
        ])?;

        descriptor.vertex.buffers = vec![vertex_layout];

        Ok(())
    }
}

pub(super) fn register(app: &mut App) {
    app.add_plugins(MaterialPlugin::<AnimatedArrayTextureMaterial>::default());
}
