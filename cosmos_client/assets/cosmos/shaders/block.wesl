import bevy_pbr::render::pbr_types;
import bevy_pbr::{
    render::pbr_fragment::pbr_input_from_standard_material,
    render::pbr_functions::alpha_discard,
    render::mesh_bindings::mesh,
    render::mesh_functions,
    render::skinning,
    render::morph::morph,
    render::view_transformations::position_world_to_clip,
};

import bevy_pbr::{
    render::forward_io::{VertexOutput, Vertex, FragmentOutput},
    render::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    render::pbr_types::{STANDARD_MATERIAL_FLAGS_UNLIT_BIT, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE},
};

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

// Semi based on https://github.com/RyanSpaker/CornGame/blob/35f600e7065d9c4f0ef294f4699b40ae69b7fb1b/shaders/corn/render/vertex.wgsl
// and https://github.com/bevyengine/bevy/blob/v0.16.0/crates/bevy_pbr/src/render/pbr.wgsl

struct ExtendedMesh {
    @location(20) @interpolate(flat) texture_index: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var my_array_texture: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var my_array_texture_sampler: sampler;

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @if(VERTEX_UVS_B)
    @location(3) uv_b: vec2<f32>,

    @if(VERTEX_TANGENTS)
    @location(4) world_tangent: vec4<f32>,
    @if(VERTEX_COLORS)
    @location(5) color: vec4<f32>,
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    @location(6) @interpolate(flat) instance_index: u32,
    @if(VISIBILITY_RANGE_DITHER)
    @location(7) @interpolate(flat) visibility_range_dither: i32,

    @location(20) @interpolate(flat) texture_index: u32,
}

@vertex
fn vertex(vertex_no_morph: Vertex, extended_mesh: ExtendedMesh) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    @if(SKINNED)
    var world_from_local = skinning::skin_model(vertex_no_morph.joint_indices, vertex_no_morph.joint_weights);
    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    @if(!SKINNED)
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    @if((VERTEX_NORMALS) && (SKINNED))
    out.world_normal = skinning::skin_normals(world_from_local, vertex_no_morph.normal);
    @if((VERTEX_NORMALS) && (!(SKINNED)))
    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex_no_morph.normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );

    @if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));

    @if(VERTEX_POSITIONS)
    out.position = position_world_to_clip(out.world_position.xyz);

    out.uv = vertex_no_morph.uv;

    @if(VERTEX_UVS_B)
    out.uv_b = vertex_no_morph.uv_b;

    @if(VERTEX_TANGENTS)
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex_no_morph.tangent,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );

    @if(VERTEX_COLORS)
    out.color = vertex_no_morph.color;

    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index = vertex_no_morph.instance_index;

    out.texture_index = extended_mesh.texture_index;

    return out;
}

@fragment
fn fragment(
    custom: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // TODO: Figure out how to do this without copying a bunch of data
    var in: VertexOutput;
    in.position = custom.position;
    in.world_position = custom.world_position;
    in.world_normal = custom.world_normal;
    in.uv = custom.uv;
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @if(VERTEX_UVS_B)
    in.uv_b = custom.uv_b;
    @if(VERTEX_TANGENTS)
    in.world_tangent = custom.world_tangent;
    @if(VERTEX_COLORS)
    in.color = custom.color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    in.instance_index = custom.instance_index;
    @if(VISIBILITY_RANGE_DITHER)
    in.visibility_range_dither = custom.visibility_range_dither;

    in.instance_index = custom.instance_index;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color *= textureSample(my_array_texture, my_array_texture_sampler, in.uv, custom.texture_index);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material.flags, pbr_input.material.alpha_cutoff, pbr_input.material.base_color);
    var out: FragmentOutput;
    // apply lighting
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    // OIT consumes premultiplied colors for blend, premultiplied, and additive materials.
    @if(MATERIAL_OIT_ENABLED) {
        let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
        if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
            oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
            discard;
        }
        if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
            oit_draw(in.position, out.color);
            discard;
        }
    }

    return out;
}
