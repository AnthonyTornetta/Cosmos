#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::morph,
    view_transformations::position_world_to_clip,
    prepass_io::{VertexOutput, Vertex, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}

const PREMULTIPLIED_ALPHA_CUTOFF = 0.5;

// We can use a simplified version of alpha_discard() here since we only need to handle the alpha_cutoff
fn prepass_alpha_discard(in: CustomVertexOutput) {
    var sample = textureSample(my_array_texture, my_array_texture_sampler, in.uv, in.texture_index);

    if sample.a < PREMULTIPLIED_ALPHA_CUTOFF {
        discard;
    }
}

struct ExtendedMesh {
    @location(20) texture_index: u32,
}

@group(2) @binding(101)
var my_array_texture: texture_2d_array<f32>;
@group(2) @binding(102)
var my_array_texture_sampler: sampler;

// Semi based on https://github.com/DarkZek/RustCraft/blob/master/assets/shaders/extended_material.wgsl

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(3) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
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

// Most of these attributes are not used in the default prepass fragment shader, but they are still needed so we can
// pass them to custom prepass shaders like pbr_prepass.wgsl.
struct CustomVertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,

#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
    @location(1) normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#endif // NORMAL_PREPASS_OR_DEFERRED_PREPASS

#ifdef VERTEX_COLORS
    @location(6) color: vec4<f32>,
#endif
}

@vertex
fn vertex(vertex_no_morph: CustomVertex, extended_mesh: ExtendedMesh) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));

        out.position = position_world_to_clip(out.world_position.xyz);
    #endif

    out.uv = vertex_no_morph.uv;

    #ifdef VERTEX_COLORS
        out.color = vertex.color;
    #endif

    out.texture_index = extended_mesh.texture_index;

    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(
    custom: CustomVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    prepass_alpha_discard(custom);

    var out: FragmentOutput;

    return out;
}
#else
@fragment
fn fragment(
    custom: CustomVertexOutput,
) {
    prepass_alpha_discard(custom);
}
#endif

