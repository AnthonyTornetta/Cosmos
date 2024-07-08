#import bevy_pbr::{
    pbr_prepass_functions,
    pbr_bindings::material,
    pbr_types,
    pbr_functions,
    prepass_io::FragmentOutput,
    mesh_view_bindings::view,
    pbr_bindings,
    skinning,
    morph,
    morph::morph,
    mesh_functions,
    view_transformations::position_world_to_clip,
}
#import bevy_render::instance_index::get_instance_index
#import bevy_render::globals::Globals

#ifdef DEFERRED_PREPASS
#import bevy_pbr::rgb9e5
#endif

@group(0) @binding(1)
var<uniform> globals: Globals;

@group(2) @binding(1)
var my_array_texture: texture_2d_array<f32>;
@group(2) @binding(2)
var my_array_texture_sampler: sampler;

// // Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// // pass them to custom prepass shaders like pbr_prepass.wgsl.
// struct Vertex {
//     @builtin(instance_index) instance_index: u32,
// #ifdef VERTEX_POSITIONS
//     @location(0) position: vec3<f32>,
// #endif
// #ifdef VERTEX_NORMALS
//     @location(1) normal: vec3<f32>,
// #endif
// #ifdef VERTEX_UVS_A
//     @location(2) uv: vec2<f32>,
// #endif
// #ifdef VERTEX_UVS_B
//     @location(3) uv_b: vec2<f32>,
// #endif
// #ifdef VERTEX_TANGENTS
//     @location(4) tangent: vec4<f32>,
// #endif
// #ifdef VERTEX_COLORS
//     @location(5) color: vec4<f32>,
// #endif
// #ifdef SKINNED
//     @location(6) joint_indices: vec4<u32>,
//     @location(7) joint_weights: vec4<f32>,
// #endif
// #ifdef MORPH_TARGETS
//     @builtin(vertex_index) index: u32,
// #endif

//     @location(20) texture_index: u32,
//     @location(21) animation_data: u32,
// };

// struct VertexOutput {
//     // This is `clip position` when the struct is used as a vertex stage output
//     // and `frag coord` when used as a fragment stage input
//     @builtin(position) position: vec4<f32>,

// #ifdef VERTEX_UVS
//     @location(0) uv: vec2<f32>,
// #endif

// #ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
//     @location(1) world_normal: vec3<f32>,
// #ifdef VERTEX_TANGENTS
//     @location(2) world_tangent: vec4<f32>,
// #endif
// #endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

//     @location(3) world_position: vec4<f32>,
// #ifdef MOTION_VECTOR_PREPASS
//     @location(4) previous_world_position: vec4<f32>,
// #endif

// #ifdef DEPTH_CLAMP_ORTHO
//     @location(5) clip_position_unclamped: vec4<f32>,
// #endif // DEPTH_CLAMP_ORTHO
// #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
//     @location(6) instance_index: u32,
// #endif

// #ifdef VERTEX_COLORS
//     @location(7) color: vec4<f32>,
// #endif

//     @location(20) texture_index: u32,
// }

// #ifdef MORPH_TARGETS
// fn morph_vertex(vertex_in: Vertex) -> Vertex {
//     var vertex = vertex_in;
//     let weight_count = morph::layer_count();
//     for (var i: u32 = 0u; i < weight_count; i ++) {
//         let weight = morph::weight_at(i);
//         if weight == 0.0 {
//             continue;
//         }
//         vertex.position += weight * morph::morph(vertex.index, morph::position_offset, i);
// #ifdef VERTEX_NORMALS
//         vertex.normal += weight * morph::morph(vertex.index, morph::normal_offset, i);
// #endif
// #ifdef VERTEX_TANGENTS
//         vertex.tangent += vec4(weight * morph::morph(vertex.index, morph::tangent_offset, i), 0.0);
// #endif
//     }
//     return vertex;
// }
// #endif

// @vertex
// fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
//     var out: VertexOutput;

// #ifdef MORPH_TARGETS
//     var vertex = morph::morph_vertex(vertex_no_morph);
// #else
//     var vertex = vertex_no_morph;
// #endif

// #ifdef SKINNED
//     var model = skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
// #else // SKINNED
//     // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//     // See https://github.com/gfx-rs/naga/issues/2416
//     var model = mesh_functions::get_model_matrix(vertex_no_morph.instance_index);
// #endif // SKINNED

//     out.position = mesh_functions::mesh_position_local_to_clip(model, vec4(vertex.position, 1.0));
// #ifdef DEPTH_CLAMP_ORTHO
//     out.clip_position_unclamped = out.position;
//     out.position.z = min(out.position.z, 1.0);
// #endif // DEPTH_CLAMP_ORTHO

// #ifdef VERTEX_UVS
//     out.uv = vertex.uv;
// #endif // VERTEX_UVS

// #ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
// #ifdef SKINNED
//     out.world_normal = skinning::skin_normals(model, vertex.normal);
// #else // SKINNED
//     out.world_normal = mesh_functions::mesh_normal_local_to_world(
//         vertex.normal,
//         // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//         // See https://github.com/gfx-rs/naga/issues/2416
//         vertex_no_morph.instance_index
//     );
// #endif // SKINNED

// #ifdef VERTEX_TANGENTS
//     out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
//         model,
//         vertex.tangent,
//         // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//         // See https://github.com/gfx-rs/naga/issues/2416
//         vertex_no_morph.instance_index
//     );
// #endif // VERTEX_TANGENTS
// #endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

// #ifdef VERTEX_COLORS
//     out.color = vertex.color;
// #endif

// #ifdef MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS
//     out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
// #endif // MOTION_VECTOR_PREPASS_OR_DEFERRED_PREPASS

// #ifdef MOTION_VECTOR_PREPASS
//     // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//     // See https://github.com/gfx-rs/naga/issues/2416
//     out.previous_world_position = mesh_functions::mesh_position_local_to_world(
//         mesh_functions::get_previous_model_matrix(vertex_no_morph.instance_index),
//         vec4<f32>(vertex.position, 1.0)
//     );
// #endif // MOTION_VECTOR_PREPASS

// #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
//     // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
//     // See https://github.com/gfx-rs/naga/issues/2416
//     out.instance_index = vertex_no_morph.instance_index;
// #endif

//     var frame_duration_ms = f32(vertex.animation_data >> u32(16)) / 1000.0;
//     var n_frames = vertex.animation_data & u32(0xFFFF);

//     var texture_index_offset = u32(globals.time / frame_duration_ms) % n_frames;

//     out.texture_index = vertex.texture_index + texture_index_offset;

//     return out;
// }

// // From https://github.com/bevyengine/bevy/blob/v0.12.0/crates/bevy_pbr/src/render/pbr_prepass.wgsl
// #ifdef PREPASS_FRAGMENT
// @fragment
// fn fragment(
//     in: VertexOutput,
//     @builtin(front_facing) is_front: bool,
// ) -> FragmentOutput {
//     prepass_alpha_discard(in);

//     var out: FragmentOutput;

// #ifdef DEPTH_CLAMP_ORTHO
//     out.frag_depth = in.clip_position_unclamped.z;
// #endif // DEPTH_CLAMP_ORTHO

// #ifdef NORMAL_PREPASS
//     // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
//     if (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
//         let double_sided = (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

//         let world_normal = pbr_functions::prepare_world_normal(
//             in.world_normal,
//             double_sided,
//             is_front,
//         );

//         let normal = pbr_functions::apply_normal_mapping(
//             material.flags,
//             world_normal,
//             double_sided,
//             is_front,
// #ifdef VERTEX_TANGENTS
// #ifdef STANDARDMATERIAL_NORMAL_MAP
//             in.world_tangent,
// #endif // STANDARDMATERIAL_NORMAL_MAP
// #endif // VERTEX_TANGENTS
// #ifdef VERTEX_UVS
//             in.uv,
// #endif // VERTEX_UVS
//             view.mip_bias,
//         );

//         out.normal = vec4(normal * 0.5 + vec3(0.5), 1.0);
//     } else {
//         out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
//     }
// #endif // NORMAL_PREPASS

// #ifdef MOTION_VECTOR_PREPASS
//     out.motion_vector = pbr_prepass_functions::calculate_motion_vector(in.world_position, in.previous_world_position);
// #endif

//     return out;
// }
// #else
// @fragment
// fn fragment(in: VertexOutput) {
//     prepass_alpha_discard(in);
// }
// #endif // PREPASS_FRAGMENT





// // From https://github.com/bevyengine/bevy/blob/v0.12.0/crates/bevy_pbr/src/render/pbr_prepass_functions.wgsl

// // Cutoff used for the premultiplied alpha modes BLEND and ADD.
// const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// // We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
// fn prepass_alpha_discard(in: VertexOutput) {

// #ifdef MAY_DISCARD
//     var output_color: vec4<f32> = pbr_bindings::material.base_color;

// #ifdef VERTEX_UVS
//     if (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
//         output_color = output_color * textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.texture_index);
//     }
// #endif // VERTEX_UVS

//     let alpha_mode = pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
//     if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
//         if output_color.a < pbr_bindings::material.alpha_cutoff {
//             discard;
//         }
//     } else if (alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD) {
//         if output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
//             discard;
//         }
//     } else if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
//         if all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
//             discard;
//         }
//     }

// #endif // MAY_DISCARD
// }















// NEW

// https://github.com/bevyengine/bevy/tree/main/crates/bevy_pbr/src/render/pre

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef VERTEX_UVS_A
    @location(1) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(2) uv_b: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(3) normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif

#ifdef VERTEX_COLORS
    @location(7) color: vec4<f32>,
#endif

#ifdef MORPH_TARGETS
    @builtin(vertex_index) index: u32,
#endif // MORPH_TARGETS

    @location(20) texture_index: u32,
    @location(21) animation_data: u32,
}



struct VertexOutput {
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,

#ifdef VERTEX_UVS_A
    @location(0) uv: vec2<f32>,
#endif

#ifdef VERTEX_UVS_B
    @location(1) uv_b: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(2) world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

    @location(4) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(5) previous_world_position: vec4<f32>,
#endif

#ifdef DEPTH_CLAMP_ORTHO
    @location(6) clip_position_unclamped: vec4<f32>,
#endif // DEPTH_CLAMP_ORTHO
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(7) instance_index: u32,
#endif

#ifdef VERTEX_COLORS
    @location(8) color: vec4<f32>,
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

    var frame_duration_ms = f32(vertex.animation_data >> u32(16)) / 1000.0;
    var n_frames = vertex.animation_data & u32(0xFFFF);

    var texture_index_offset = u32(globals.time / frame_duration_ms) % n_frames;

    out.texture_index = vertex.texture_index + texture_index_offset;

    return out;
}

// @fragment
// fn fragment(
//     mesh: VertexOutput,
// ) -> @location(0) vec4<f32> {
// #ifdef VERTEX_COLORS
//     return mesh.color;
// #else
//     return vec4<f32>(1.0, 0.0, 1.0, 1.0);
// #endif
// }








// @fragment
// fn fragment(
//     in: VertexOutput,
//     @builtin(front_facing) is_front: bool,
// ) -> FragmentOutput {
//     // If we're in the crossfade section of a visibility range, conditionally
//     // discard the fragment according to the visibility pattern.
// #ifdef VISIBILITY_RANGE_DITHER
//     pbr_functions::visibility_range_dither(in.position, in.visibility_range_dither);
// #endif

//     // generate a PbrInput struct from the StandardMaterial bindings
//     var pbr_input = pbr_input_from_standard_material(in, is_front);

//     // alpha discard
//     pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

// #ifdef PREPASS_PIPELINE
//     // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
//     let out = deferred_output(in, pbr_input);
// #else
//     // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
//     // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
//     var out: FragmentOutput;
//     if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
//         out.color = apply_pbr_lighting(pbr_input);
//     } else {
//         out.color = pbr_input.material.base_color;
//     }

//     // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
//     // note this does not include fullscreen postprocessing effects like bloom.
//     out.color = main_pass_post_lighting_processing(pbr_input, out.color);
// #endif

//     return out;
// }






















#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    prepass_alpha_discard(in);

    var out: FragmentOutput;

#ifdef DEPTH_CLAMP_ORTHO
    out.frag_depth = in.clip_position_unclamped.z;
#endif // DEPTH_CLAMP_ORTHO

#ifdef NORMAL_PREPASS
    // NOTE: Unlit bit not set means == 0 is true, so the true case is if lit
    if (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        let double_sided = (material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u;

        let world_normal = pbr_functions::prepare_world_normal(
            in.world_normal,
            double_sided,
            is_front,
        );

        var normal = world_normal;

#ifdef VERTEX_UVS
#ifdef VERTEX_TANGENTS
#ifdef STANDARD_MATERIAL_NORMAL_MAP

#ifdef STANDARD_MATERIAL_NORMAL_MAP_UV_B
        let uv = (material.uv_transform * vec3(in.uv_b, 1.0)).xy;
#else
        let uv = (material.uv_transform * vec3(in.uv, 1.0)).xy;
#endif

        // Fill in the sample bias so we can sample from textures.
        var bias: SampleBias;
#ifdef MESHLET_MESH_MATERIAL_PASS
        bias.ddx_uv = in.ddx_uv;
        bias.ddy_uv = in.ddy_uv;
#else   // MESHLET_MESH_MATERIAL_PASS
        bias.mip_bias = view.mip_bias;
#endif  // MESHLET_MESH_MATERIAL_PASS

        let Nt = pbr_functions::sample_texture(
            pbr_bindings::normal_map_texture,
            pbr_bindings::normal_map_sampler,
            uv,
            bias,
        ).rgb;
        let TBN = pbr_functions::calculate_tbn_mikktspace(normal, in.world_tangent);

        normal = pbr_functions::apply_normal_mapping(
            material.flags,
            TBN,
            double_sided,
            is_front,
            Nt,
        );

#endif  // STANDARD_MATERIAL_NORMAL_MAP
#endif  // VERTEX_TANGENTS
#endif  // VERTEX_UVS

        out.normal = vec4(normal * 0.5 + vec3(0.5), 1.0);
    } else {
        out.normal = vec4(in.world_normal * 0.5 + vec3(0.5), 1.0);
    }
#endif // NORMAL_PREPASS

#ifdef MOTION_VECTOR_PREPASS
#ifdef MESHLET_MESH_MATERIAL_PASS
    out.motion_vector = in.motion_vector;
#else
    out.motion_vector = pbr_prepass_functions::calculate_motion_vector(in.world_position, in.previous_world_position);
#endif
#endif

    return out;
}
#else
@fragment
fn fragment(in: VertexOutput) {
    prepass_alpha_discard(in);
}
#endif







// Cutoff used for the premultiplied alpha modes BLEND, ADD, and ALPHA_TO_COVERAGE.
const PREMULTIPLIED_ALPHA_CUTOFF = 0.05;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: VertexOutput) {

#ifdef MAY_DISCARD
    var output_color: vec4<f32> = pbr_bindings::material.base_color;

#ifdef VERTEX_UVS
#ifdef STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv_b;
#else   // STANDARD_MATERIAL_BASE_COLOR_UV_B
    var uv = in.uv;
#endif  // STANDARD_MATERIAL_BASE_COLOR_UV_B

    let uv_transform = pbr_bindings::material.uv_transform;
    uv = (uv_transform * vec3(uv, 1.0)).xy;
    if (pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u {
        output_color = output_color * textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.texture_index);
    }
#endif // VERTEX_UVS
    let alpha_mode = pbr_bindings::material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK {
        if output_color.a < pbr_bindings::material.alpha_cutoff {
            discard;
        }
    } else if (alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD ||
            alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ALPHA_TO_COVERAGE) {
        if output_color.a < PREMULTIPLIED_ALPHA_CUTOFF {
            discard;
        }
    } else if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED {
        if all(output_color < vec4(PREMULTIPLIED_ALPHA_CUTOFF)) {
            discard;
        }
    }

#endif // MAY_DISCARD
}