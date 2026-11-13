import bevy_pbr::{
    render::pbr_fragment::pbr_input_from_standard_material,
    render::pbr_functions::alpha_discard,
    render::mesh_bindings::mesh,
    render::mesh_functions,
    render::skinning,
    render::morph::morph,
    render::view_transformations::position_world_to_clip,
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};

import bevy_render::globals::Globals;

@group(0) @binding(1)
var<uniform> globals: Globals;

// Semi based on https://github.com/RyanSpaker/CornGame/blob/35f600e7065d9c4f0ef294f4699b40ae69b7fb1b/shaders/corn/render/vertex.wgsl
// and https://github.com/bevyengine/bevy/blob/v0.16.0/crates/bevy_pbr/src/render/pbr.wgsl

const PREMULTIPLIED_ALPHA_CUTOFF = 0.3;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: CustomVertexOutput) {
    var sample = textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.texture_index);

    if sample.a < PREMULTIPLIED_ALPHA_CUTOFF {
        discard;
    }
}

struct ExtendedMesh {
    @location(20) @interpolate(flat) texture_index: u32,
    @location(21) @interpolate(flat) animation_data: u32,
}

@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var my_array_texture: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var my_array_texture_sampler: sampler;

// Semi based on https://github.com/DarkZek/RustCraft/blob/master/assets/shaders/extended_material.wgsl

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
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

// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wgsl.
struct CustomVertex {
    // For some reason the default bevy `Vertex` object just doesn't work here - idk why.
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

    @if(VERTEX_UVS_A)
    @location(2) uv: vec2<f32>,

    @if(NORMAL_PREPASS_OR_DEFERRED_PREPASS)
    @location(1) normal: vec3<f32>,
    @if((NORMAL_PREPASS_OR_DEFERRED_PREPASS) && (VERTEX_TANGENTS))
    @location(3) tangent: vec4<f32>,

    @if(VERTEX_COLORS)
    @location(6) color: vec4<f32>,
}

@vertex
fn vertex(vertex_no_morph: CustomVertex, extended_mesh: ExtendedMesh) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    @if(VERTEX_POSITIONS)
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));

    @if(VERTEX_POSITIONS)
    out.position = position_world_to_clip(out.world_position.xyz);

    out.uv = vertex_no_morph.uv;

    @if(VERTEX_COLORS)
    out.color = vertex_no_morph.color;

    var frame_duration_ms = f32(extended_mesh.animation_data >> u32(16)) / 1000.0;
    var n_frames = extended_mesh.animation_data & u32(0xFFFF);

    var texture_index_offset = u32(globals.time / frame_duration_ms) % n_frames;

    out.texture_index = extended_mesh.texture_index + texture_index_offset;

    return out;
}

@if(PREPASS_FRAGMENT)
@fragment
fn fragment(
    custom: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    @if(PREPASS_FRAGMENT)
    prepass_alpha_discard(custom);

    @if(PREPASS_FRAGMENT)
    var out: FragmentOutput;

    @if(PREPASS_FRAGMENT)
    return out;
}
@if(!PREPASS_FRAGMENT)
@fragment
fn fragment(
    custom: CustomVertexOutput,
) {
    @if(!PREPASS_FRAGMENT)
    prepass_alpha_discard(custom);
}
