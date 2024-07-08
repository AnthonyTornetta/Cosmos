#import bevy_pbr::{
    pbr_functions,
    pbr_functions::SampleBias,
    pbr_bindings,
    pbr_types,
    prepass_utils,
    lighting,
    mesh_bindings::mesh,
    mesh_view_bindings::{view, globals},
    parallax_mapping::parallaxed_uv,
    lightmap::lightmap,
    mesh_functions,
    view_transformations::position_world_to_clip,
}

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#import bevy_pbr::gtao_utils::gtao_multibounce
#endif

#import bevy_pbr::{
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::FragmentOutput,
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::FragmentOutput,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

@group(2) @binding(1)
var my_array_texture: texture_2d_array<f32>;
@group(2) @binding(2)
var my_array_texture_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(6) joint_indices: vec4<u32>,
    @location(7) joint_weights: vec4<f32>,
#endif
#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif

    @location(20) texture_index: u32,
};


struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif

    @location(20) texture_index: u32,
}

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex) -> Vertex {
    var vertex = vertex_in;
    let weight_count = bevy_pbr::morph::layer_count();
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::normal_offset, i);
#endif
#ifdef VERTEX_TANGENTS
        vertex.tangent += vec4(weight * bevy_pbr::morph::morph(vertex.index, bevy_pbr::morph::tangent_offset, i), 0.0);
#endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var world_from_local = bevy_pbr::skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = bevy_pbr::skinning::skin_normals(world_from_local, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = vertex_no_morph.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, world_from_local[3]);
#endif

    out.texture_index = vertex.texture_index;

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
#ifdef VISIBILITY_RANGE_DITHER
    pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
#endif

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
    let out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}

// stolen from https://github.com/bevyengine/bevy/blob/v0.14.0/crates/bevy_pbr/src/render/pbr_fragment.wgsl

// prepare a basic PbrInput from the vertex stage output, mesh binding and view binding
fn pbr_input_from_vertex_output(
    in: VertexOutput,
    is_front: bool,
    double_sided: bool,
) -> pbr_types::PbrInput {
    var pbr_input: pbr_types::PbrInput = pbr_types::pbr_input_new();

#ifdef MESHLET_MESH_MATERIAL_PASS
    pbr_input.flags = in.mesh_flags;
#else
    pbr_input.flags = mesh[in.instance_index].flags;
#endif

    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;

#ifdef VERTEX_COLORS
    pbr_input.material.base_color = in.color;
#endif

    pbr_input.world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        double_sided,
        is_front,
    );

#ifdef LOAD_PREPASS_NORMALS
    pbr_input.N = prepass_utils::prepass_normal(in.position, 0u);
#else
    pbr_input.N = normalize(pbr_input.world_normal);
#endif

    return pbr_input;
}

// stolen from https://github.com/bevyengine/bevy/blob/v0.14.0/crates/bevy_pbr/src/render/pbr_fragment.wgsl

// Prepare a full PbrInput by sampling all textures to resolve
// the material members
fn pbr_input_from_standard_material(
    in: VertexOutput,
    is_front: bool,
) -> pbr_types::PbrInput {
    let double_sided = (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

    var pbr_input: pbr_types::PbrInput = pbr_input_from_vertex_output(in, is_front, double_sided);
    pbr_input.material.flags = pbr_bindings::material.flags;
    pbr_input.material.base_color *= pbr_bindings::material.base_color;
    pbr_input.material.deferred_lighting_pass_id = pbr_bindings::material.deferred_lighting_pass_id;

    // Neubelt and Pettineo 2013, "Crafting a Next-gen Material Pipeline for The Order: 1886"
    let NdotV = max(dot(pbr_input.N, pbr_input.V), 0.0001);

    // Fill in the sample bias so we can sample from textures.
    var bias: SampleBias;
#ifdef MESHLET_MESH_MATERIAL_PASS
    bias.ddx_uv = in.ddx_uv;
    bias.ddy_uv = in.ddy_uv;
#else   // MESHLET_MESH_MATERIAL_PASS
    bias.mip_bias = view.mip_bias;
#endif  // MESHLET_MESH_MATERIAL_PASS

#ifdef VERTEX_UVS
    let uv_transform = pbr_bindings::material.uv_transform;
#ifdef VERTEX_UVS_A
    var uv = (uv_transform * vec3(in.uv, 1.0)).xy;
#endif

#ifdef VERTEX_UVS_B
    var uv_b = (uv_transform * vec3(in.uv_b, 1.0)).xy;
#else
    var uv_b = uv;
#endif

#ifdef VERTEX_TANGENTS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let V = pbr_input.V;
        let N = in.world_normal;
        let T = in.world_tangent.xyz;
        let B = in.world_tangent.w * cross(N, T);
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
#ifdef VERTEX_UVS_A
        uv = parallaxed_uv(
            pbr_bindings::material.parallax_depth_scale,
            pbr_bindings::material.max_parallax_layer_count,
            pbr_bindings::material.max_relief_mapping_search_steps,
            uv,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
        );
#endif

#ifdef VERTEX_UVS_B
        uv_b = parallaxed_uv(
            pbr_bindings::material.parallax_depth_scale,
            pbr_bindings::material.max_parallax_layer_count,
            pbr_bindings::material.max_relief_mapping_search_steps,
            uv_b,
            // Flip the direction of Vt to go toward the surface to make the
            // parallax mapping algorithm easier to understand and reason
            // about.
            -Vt,
        );
#else
        uv_b = uv;
#endif
    }
#endif // VERTEX_TANGENTS

    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        pbr_input.material.base_color *= textureSample(my_array_texture, my_array_texture_sampler, uv, in.texture_index);

#ifdef ALPHA_TO_COVERAGE
    // Sharpen alpha edges.
    //
    // https://bgolus.medium.com/anti-aliased-alpha-test-the-esoteric-alpha-to-coverage-8b177335ae4f
    let alpha_mode = pbr_bindings::material.flags &
        pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE {
        pbr_input.material.base_color.a = (pbr_input.material.base_color.a -
                pbr_bindings::material.alpha_cutoff) /
                max(fwidth(pbr_input.material.base_color.a), 0.0001) + 0.5;
    }
#endif // ALPHA_TO_COVERAGE

    }
#endif // VERTEX_UVS

    pbr_input.material.flags = pbr_bindings::material.flags;

    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u) {
        pbr_input.material.reflectance = pbr_bindings::material.reflectance;
        pbr_input.material.ior = pbr_bindings::material.ior;
        pbr_input.material.attenuation_color = pbr_bindings::material.attenuation_color;
        pbr_input.material.attenuation_distance = pbr_bindings::material.attenuation_distance;
        pbr_input.material.alpha_cutoff = pbr_bindings::material.alpha_cutoff;

        // emissive
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * pbr_functions::sample_texture(
                pbr_bindings::emissive_texture,
                pbr_bindings::emissive_sampler,
#ifdef STANDARD_MATERIAL_EMISSIVE_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).rgb, emissive.a);
        }
#endif
        pbr_input.material.emissive = emissive;

        // metallic and perceptual roughness
        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
        let roughness = lighting::perceptualRoughnessToRoughness(perceptual_roughness);
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = pbr_functions::sample_texture(
                pbr_bindings::metallic_roughness_texture,
                pbr_bindings::metallic_roughness_sampler,
#ifdef STANDARD_MATERIAL_METALLIC_ROUGHNESS_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            );
            // Sampling from GLTF standard channels for now
            metallic *= metallic_roughness.b;
            perceptual_roughness *= metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        // Clearcoat factor
        pbr_input.material.clearcoat = pbr_bindings::material.clearcoat;
#ifdef VERTEX_UVS
#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_CLEARCOAT_TEXTURE_BIT) != 0u) {
            pbr_input.material.clearcoat *= pbr_functions::sample_texture(
                pbr_bindings::clearcoat_texture,
                pbr_bindings::clearcoat_sampler,
#ifdef STANDARD_MATERIAL_CLEARCOAT_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).r;
        }
#endif  // PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
#endif  // VERTEX_UVS

        // Clearcoat roughness
        pbr_input.material.clearcoat_perceptual_roughness = pbr_bindings::material.clearcoat_perceptual_roughness;
#ifdef VERTEX_UVS
#ifdef PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_CLEARCOAT_ROUGHNESS_TEXTURE_BIT) != 0u) {
            pbr_input.material.clearcoat_perceptual_roughness *= pbr_functions::sample_texture(
                pbr_bindings::clearcoat_roughness_texture,
                pbr_bindings::clearcoat_roughness_sampler,
#ifdef STANDARD_MATERIAL_CLEARCOAT_ROUGHNESS_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).g;
        }
#endif  // PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED
#endif  // VERTEX_UVS

        var specular_transmission: f32 = pbr_bindings::material.specular_transmission;
#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT) != 0u) {
            specular_transmission *= pbr_functions::sample_texture(
                pbr_bindings::specular_transmission_texture,
                pbr_bindings::specular_transmission_sampler,
#ifdef STANDARD_MATERIAL_SPECULAR_TRANSMISSION_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).r;
        }
#endif
#endif
        pbr_input.material.specular_transmission = specular_transmission;

        var thickness: f32 = pbr_bindings::material.thickness;
#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT) != 0u) {
            thickness *= pbr_functions::sample_texture(
                pbr_bindings::thickness_texture,
                pbr_bindings::thickness_sampler,
#ifdef STANDARD_MATERIAL_THICKNESS_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).g;
        }
#endif
#endif
        // scale thickness, accounting for non-uniform scaling (e.g. a “squished” mesh)
        // TODO: Meshlet support
#ifndef MESHLET_MESH_MATERIAL_PASS
        thickness *= length(
            (transpose(mesh[in.instance_index].world_from_local) * vec4(pbr_input.N, 0.0)).xyz
        );
#endif
        pbr_input.material.thickness = thickness;

        var diffuse_transmission = pbr_bindings::material.diffuse_transmission;
#ifdef VERTEX_UVS
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT) != 0u) {
            diffuse_transmission *= pbr_functions::sample_texture(
                pbr_bindings::diffuse_transmission_texture,
                pbr_bindings::diffuse_transmission_sampler,
#ifdef STANDARD_MATERIAL_DIFFUSE_TRANSMISSION_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).a;
        }
#endif
#endif
        pbr_input.material.diffuse_transmission = diffuse_transmission;

        var diffuse_occlusion: vec3<f32> = vec3(1.0);
        var specular_occlusion: f32 = 1.0;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            diffuse_occlusion *= pbr_functions::sample_texture(
                pbr_bindings::occlusion_texture,
                pbr_bindings::occlusion_sampler,
#ifdef STANDARD_MATERIAL_OCCLUSION_UV_B
                uv_b,
#else
                uv,
#endif
                bias,
            ).r;
        }
#endif
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        diffuse_occlusion = min(diffuse_occlusion, ssao_multibounce);
        // Use SSAO to estimate the specular occlusion.
        // Lagarde and Rousiers 2014, "Moving Frostbite to Physically Based Rendering"
        specular_occlusion =  saturate(pow(NdotV + ssao, exp2(-16.0 * roughness - 1.0)) - 1.0 + ssao);
#endif
        pbr_input.diffuse_occlusion = diffuse_occlusion;
        pbr_input.specular_occlusion = specular_occlusion;

        // N (normal vector)
#ifndef LOAD_PREPASS_NORMALS

        pbr_input.N = normalize(pbr_input.world_normal);
        pbr_input.clearcoat_N = pbr_input.N;

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS

        let TBN = pbr_functions::calculate_tbn_mikktspace(pbr_input.world_normal, in.world_tangent);

#ifdef STANDARD_MATERIAL_NORMAL_MAP

        let Nt = pbr_functions::sample_texture(
            pbr_bindings::normal_map_texture,
            pbr_bindings::normal_map_sampler,
#ifdef STANDARD_MATERIAL_NORMAL_MAP_UV_B
                uv_b,
#else
                uv,
#endif
            bias,
        ).rgb;

        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            TBN,
            double_sided,
            is_front,
            Nt,
        );

#endif  // STANDARD_MATERIAL_NORMAL_MAP

#ifdef STANDARD_MATERIAL_CLEARCOAT

        // Note: `KHR_materials_clearcoat` specifies that, if there's no
        // clearcoat normal map, we must set the normal to the mesh's normal,
        // and not to the main layer's bumped normal.

#ifdef STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP

        let clearcoat_Nt = pbr_functions::sample_texture(
            pbr_bindings::clearcoat_normal_texture,
            pbr_bindings::clearcoat_normal_sampler,
#ifdef STANDARD_MATERIAL_CLEARCOAT_NORMAL_UV_B
                uv_b,
#else
                uv,
#endif
            bias,
        ).rgb;

        pbr_input.clearcoat_N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            TBN,
            double_sided,
            is_front,
            clearcoat_Nt,
        );

#endif  // STANDARD_MATERIAL_CLEARCOAT_NORMAL_MAP

#endif  // STANDARD_MATERIAL_CLEARCOAT

#endif  // VERTEX_TANGENTS
#endif  // VERTEX_UVS

        // Take anisotropy into account.
        //
        // This code comes from the `KHR_materials_anisotropy` spec:
        // <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md#individual-lights>
#ifdef PBR_ANISOTROPY_TEXTURE_SUPPORTED
#ifdef VERTEX_TANGENTS
#ifdef STANDARD_MATERIAL_ANISOTROPY

        var anisotropy_strength = pbr_bindings::material.anisotropy_strength;
        var anisotropy_direction = pbr_bindings::material.anisotropy_rotation;

        // Adjust based on the anisotropy map if there is one.
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ANISOTROPY_TEXTURE_BIT) != 0u) {
            let anisotropy_texel = pbr_functions::sample_texture(
                pbr_bindings::anisotropy_texture,
                pbr_bindings::anisotropy_sampler,
#ifdef STANDARD_MATERIAL_ANISOTROPY_UV_B
                uv_b,
#else   // STANDARD_MATERIAL_ANISOTROPY_UV_B
                uv,
#endif  // STANDARD_MATERIAL_ANISOTROPY_UV_B
                bias,
            ).rgb;

            let anisotropy_direction_from_texture = normalize(anisotropy_texel.rg * 2.0 - 1.0);
            // Rotate by the anisotropy direction.
            anisotropy_direction =
                mat2x2(anisotropy_direction.xy, anisotropy_direction.yx * vec2(-1.0, 1.0)) *
                anisotropy_direction_from_texture;
            anisotropy_strength *= anisotropy_texel.b;
        }

        pbr_input.anisotropy_strength = anisotropy_strength;

        let anisotropy_T = normalize(TBN * vec3(anisotropy_direction, 0.0));
        let anisotropy_B = normalize(cross(pbr_input.world_normal, anisotropy_T));
        pbr_input.anisotropy_T = anisotropy_T;
        pbr_input.anisotropy_B = anisotropy_B;

#endif  // STANDARD_MATERIAL_ANISOTROPY
#endif  // VERTEX_TANGENTS
#endif  // PBR_ANISOTROPY_TEXTURE_SUPPORTED

#endif  // LOAD_PREPASS_NORMALS

// TODO: Meshlet support
#ifdef LIGHTMAP
        pbr_input.lightmap_light = lightmap(
            in.uv_b,
            pbr_bindings::material.lightmap_exposure,
            in.instance_index);
#endif
    }

    return pbr_input;
}