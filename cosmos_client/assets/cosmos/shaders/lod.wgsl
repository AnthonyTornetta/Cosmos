#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::morph,
    view_transformations::position_world_to_clip,
}

#import bevy_pbr::{
    forward_io::{VertexOutput, Vertex, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::{STANDARD_MATERIAL_FLAGS_UNLIT_BIT, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS, STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE},
}

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // OIT_ENABLED

// Semi based on https://github.com/RyanSpaker/CornGame/blob/35f600e7065d9c4f0ef294f4699b40ae69b7fb1b/shaders/corn/render/vertex.wgsl
// and https://github.com/bevyengine/bevy/blob/v0.16.0/crates/bevy_pbr/src/render/pbr.wgsl

struct ExtendedMesh {
    @location(1) packed_data: u32,    
}

@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var my_array_texture: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var my_array_texture_sampler: sampler;

struct CustomVertexOutput {
    @builtin(position) position: vec4<f32>,
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

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

@vertex
fn vertex(vertex_no_morph: Vertex, extended_mesh: ExtendedMesh) -> CustomVertexOutput {
    var out: CustomVertexOutput;

    let normal_bits = extended_mesh.packed_data >> 29;
    let uv_bits = extended_mesh.packed_data >> 27 & 3;

    // var normal: vec3<f32> = vec3(0.0, 0.0, 0.0);    
    // /*
    //     BlockDirection::PosX.index() == 0
    //  && BlockDirection::NegX.index() == 1
    //  && BlockDirection::PosY.index() == 2
    //  && BlockDirection::NegY.index() == 3
    //  && BlockDirection::PosZ.index() == 4
    //  && BlockDirection::NegZ.index() == 5
    // */
    // switch(normal_bits) {
    //     case u32(0): {
    //         normal.x = 1.0;
    //         break;
    //     }
    //     case u32(1): {
    //         normal.x = -1.0;
    //         break;
    //     }
    //     case u32(2): {
    //         normal.y = 1.0;
    //         break;
    //     }
    //     case u32(3): {
    //         normal.y = -1.0;
    //         break;
    //     }
    //     case u32(4): {
    //         normal.z = 1.0;
    //         break;
    //     }
    //     default: {
    //         normal.z = -1.0;
    //         break;
    //     }
    // }

    let uvs = vec2<f32>(f32(uv_bits >> 1), f32(uv_bits & 1));
    let normal_axis = extended_mesh.packed_data >> 30;
    let normal_negative = (extended_mesh.packed_data >> 29) & 1;
    var normal: vec3<f32> = vec3(0.0, 0.0, 0.0);
    normal[normal_axis] = 1.0 - f32(normal_negative << 1);
    let texture_index = extended_mesh.packed_data & 134217727; // 134217727 == 0b00000111_11111111_11111111_11111111

    // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
    // See https://github.com/gfx-rs/naga/issues/2416 .
    var world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        normal,
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        vertex_no_morph.instance_index
    );

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));

        out.position = position_world_to_clip(out.world_position.xyz);
    #endif

    out.uv = uvs;

    #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        // Use vertex_no_morph.instance_index instead of vertex.instance_index to work around a wgpu dx12 bug.
        // See https://github.com/gfx-rs/naga/issues/2416
        out.instance_index = vertex_no_morph.instance_index;
    #endif

    out.texture_index = texture_index;

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
    // This is `clip position` when the struct is used as a vertex stage output
    // and `frag coord` when used as a fragment stage input
#ifdef VERTEX_UVS_B
    in.uv_b = custom.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    in.world_tangent = custom.world_tangent;
#endif
#ifdef VERTEX_COLORS
    in.color = custom.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    in.instance_index = custom.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    in.visibility_range_dither = custom.visibility_range_dither;
#endif

    in.instance_index = custom.instance_index;

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color *= textureSample(my_array_texture, my_array_texture_sampler, custom.uv, custom.texture_index);
    
    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
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

    // OIT is transparent stuff
#ifdef OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED

    return out;
}

