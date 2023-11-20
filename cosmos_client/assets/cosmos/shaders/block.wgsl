#import bevy_pbr::{
    mesh_functions,
    skinning,
    morph::morph,
    view_transformations::position_world_to_clip,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_functions,
    pbr_bindings,
    pbr_types,
    forward_io::FragmentOutput,
    prepass_utils,
    mesh_bindings::mesh,
    mesh_view_bindings::view,
    parallax_mapping::parallaxed_uv,
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#import bevy_render::instance_index::get_instance_index

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::mesh_view_bindings::screen_space_ambient_occlusion_texture
#import bevy_pbr::gtao_utils::gtao_multibounce
#endif

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
#endif

struct Vertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
// (Alternate UVs are at location 3, but they're currently unused here.)
// #ifdef VERTEX_TANGENTS
//     @location(4) tangent: vec4<f32>,
// #endif
// #ifdef VERTEX_COLORS
//     @location(5) color: vec4<f32>,
// #endif
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
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
// #ifdef VERTEX_TANGENTS
//     @location(3) world_tangent: vec4<f32>,
// #endif
// #ifdef VERTEX_COLORS
//     @location(4) color: vec4<f32>,
// #endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(5) @interpolate(flat) instance_index: u32,
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
        vertex.position += weight * morph(vertex.index, bevy_pbr::morph::position_offset, i);
#ifdef VERTEX_NORMALS
        vertex.normal += weight * morph(vertex.index, bevy_pbr::morph::normal_offset, i);
#endif
// #ifdef VERTEX_TANGENTS
//         vertex.tangent += vec4(weight * morph(vertex.index, bevy_pbr::morph::tangent_offset, i), 0.0);
// #endif
    }
    return vertex;
}
#endif

@group(1) @binding(1)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(2)
var my_array_texture_sampler: sampler;


// Stolen from: https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/mesh.wgsl
@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var model = skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var model = mesh_functions::get_model_matrix(vertex_no_morph.instance_index);
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(model, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        get_instance_index(vertex_no_morph.instance_index)
    );
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

// #ifdef VERTEX_TANGENTS
//     out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
//         model,
//         vertex.tangent,
//         // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//         // See https://github.com/gfx-rs/naga/issues/2416
//         get_instance_index(vertex_no_morph.instance_index)
//     );
// #endif

// #ifdef VERTEX_COLORS
//     out.color = vertex.color;
// #endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    out.instance_index = get_instance_index(vertex_no_morph.instance_index);
#endif

    out.texture_index = vertex.texture_index;

    return out;
}










// FRAGMENT



// prepare a basic PbrInput from the vertex stage output, mesh binding and view binding
fn pbr_input_from_vertex_output(
    in: VertexOutput,
    is_front: bool,
    double_sided: bool,
) -> pbr_types::PbrInput {
    var pbr_input: pbr_types::PbrInput = pbr_types::pbr_input_new();

    pbr_input.flags = mesh[in.instance_index].flags;
    pbr_input.is_orthographic = view.projection[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(in.world_position, pbr_input.is_orthographic);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;

// #ifdef VERTEX_COLORS
//     pbr_input.material.base_color = in.color;
// #endif

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

#ifdef VERTEX_UVS
    var uv = in.uv;

#ifdef VERTEX_TANGENTS
    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DEPTH_MAP_BIT) != 0u) {
        let V = pbr_input.V;
        let N = in.world_normal;
        let T = in.world_tangent.xyz;
        let B = in.world_tangent.w * cross(N, T);
        // Transform V from fragment to camera in world space to tangent space.
        let Vt = vec3(dot(V, T), dot(V, B), dot(V, N));
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
    }
#endif // VERTEX_TANGENTS

    if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        pbr_input.material.base_color *= textureSample(my_array_texture, my_array_texture_sampler, uv, in.texture_index);
        // pbr_input.material.base_color *= textureSampleBias(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv, view.mip_bias);
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
        // TODO use .a for exposure compensation in HDR
        var emissive: vec4<f32> = pbr_bindings::material.emissive;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_EMISSIVE_TEXTURE_BIT) != 0u) {
            emissive = vec4<f32>(emissive.rgb * textureSampleBias(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, uv, view.mip_bias).rgb, 1.0);
        }
#endif
        pbr_input.material.emissive = emissive;

        // metallic and perceptual roughness
        var metallic: f32 = pbr_bindings::material.metallic;
        var perceptual_roughness: f32 = pbr_bindings::material.perceptual_roughness;
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_METALLIC_ROUGHNESS_TEXTURE_BIT) != 0u) {
            let metallic_roughness = textureSampleBias(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, uv, view.mip_bias);
            // Sampling from GLTF standard channels for now
            metallic *= metallic_roughness.b;
            perceptual_roughness *= metallic_roughness.g;
        }
#endif
        pbr_input.material.metallic = metallic;
        pbr_input.material.perceptual_roughness = perceptual_roughness;

        var specular_transmission: f32 = pbr_bindings::material.specular_transmission;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_SPECULAR_TRANSMISSION_TEXTURE_BIT) != 0u) {
            specular_transmission *= textureSample(pbr_bindings::specular_transmission_texture, pbr_bindings::specular_transmission_sampler, uv).r;
        }
#endif
        pbr_input.material.specular_transmission = specular_transmission;

        var thickness: f32 = pbr_bindings::material.thickness;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_THICKNESS_TEXTURE_BIT) != 0u) {
            thickness *= textureSample(pbr_bindings::thickness_texture, pbr_bindings::thickness_sampler, uv).g;
        }
#endif
        // scale thickness, accounting for non-uniform scaling (e.g. a “squished” mesh)
        thickness *= length(
            (transpose(mesh[in.instance_index].model) * vec4(pbr_input.N, 0.0)).xyz
        );
        pbr_input.material.thickness = thickness;

        var diffuse_transmission = pbr_bindings::material.diffuse_transmission;
#ifdef PBR_TRANSMISSION_TEXTURES_SUPPORTED
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DIFFUSE_TRANSMISSION_TEXTURE_BIT) != 0u) {
            diffuse_transmission *= textureSample(pbr_bindings::diffuse_transmission_texture, pbr_bindings::diffuse_transmission_sampler, uv).a;
        }
#endif
        pbr_input.material.diffuse_transmission = diffuse_transmission;

        // occlusion
        // TODO: Split into diffuse/specular occlusion?
        var occlusion: vec3<f32> = vec3(1.0);
#ifdef VERTEX_UVS
        if ((pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_OCCLUSION_TEXTURE_BIT) != 0u) {
            occlusion = vec3(textureSampleBias(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, uv, view.mip_bias).r);
        }
#endif
#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
        let ssao = textureLoad(screen_space_ambient_occlusion_texture, vec2<i32>(in.position.xy), 0i).r;
        let ssao_multibounce = gtao_multibounce(ssao, pbr_input.material.base_color.rgb);
        occlusion = min(occlusion, ssao_multibounce);
#endif
        pbr_input.occlusion = occlusion;

        // N (normal vector)
#ifndef LOAD_PREPASS_NORMALS
        pbr_input.N = pbr_functions::apply_normal_mapping(
            pbr_bindings::material.flags,
            pbr_input.world_normal,
            double_sided,
            is_front,
// #ifdef VERTEX_TANGENTS
// #ifdef STANDARDMATERIAL_NORMAL_MAP
//             in.world_tangent,
// #endif
// #endif
#ifdef VERTEX_UVS
            uv,
#endif
            view.mip_bias,
        );
#endif
    }

    return pbr_input;
}





@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
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

    // var out: FragmentOutput;
    // out.color = vec4(1.0, 0.5, 0.0, 1.0);

    return out;
}
