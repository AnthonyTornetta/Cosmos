#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_types             STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT
#import bevy_core_pipeline::tonemapping tone_mapping
#import bevy_pbr::pbr_functions as fns
#import bevy_pbr::mesh_bindings         mesh
#import bevy_pbr::mesh_functions as mesh_functions

@group(1) @binding(0)
var my_array_texture: texture_2d_array<f32>;
@group(1) @binding(1)
var my_array_texture_sampler: sampler;

struct CustomMeshVertexOutput {
    // this is `clip position` when the struct is used as a vertex stage output 
    // and `frag coord` when used as a fragment stage input
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    #ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
    #endif
    #ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
    #endif
    #ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
    #endif

    @location(5) texture_index: u32,
}


struct CustomVertex {
    #ifdef VERTEX_POSITIONS
        @location(0) position: vec3<f32>,
    #endif
    #ifdef VERTEX_NORMALS
        @location(1) normal: vec3<f32>,
    #endif
    #ifdef VERTEX_UVS
        @location(2) uv: vec2<f32>,
    #endif
    #ifdef VERTEX_TANGENTS
        @location(3) tangent: vec4<f32>,
    #endif
    #ifdef VERTEX_COLORS

    @location(4) color: vec4<f32>,

    #endif
        @location(5) texture_index: u32,
    #ifdef MORPH_TARGETS
        @builtin(vertex_index) index: u32,
    #endif
};

@vertex
// Stolen from: https://github.com/bevyengine/bevy/blob/aeeb20ec4c43db07c2690ce926169ed9d6550d93/crates/bevy_pbr/src/render/mesh.wgsl
fn vertex(vertex_no_morph: CustomVertex) -> CustomMeshVertexOutput {
    var out: CustomMeshVertexOutput;

#ifdef MORPH_TARGETS
    var vertex = morph_vertex(vertex_no_morph);
#else
    var vertex = vertex_no_morph;
#endif

#ifdef SKINNED
    var model = bevy_pbr::skinning::skin_model(vertex.joint_indices, vertex.joint_weights);
#else
    var model = mesh.model;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = bevy_pbr::skinning::skin_normals(model, vertex.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal);
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.position = mesh_functions::mesh_position_world_to_clip(out.world_position);
#endif

#ifdef VERTEX_UVS
    out.uv = vertex.uv;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(model, vertex.tangent);
#endif

#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    out.texture_index = vertex.texture_index;

    return out;
}


@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: CustomMeshVertexOutput,
) -> @location(0) vec4<f32> {
    let layer = i32(mesh.world_position.x) & 0x3;

    // Prepare a 'processed' StandardMaterial by sampling all textures to resolve
    // the material members
    var pbr_input: fns::PbrInput = fns::pbr_input_new();

    var abc = vec2(mesh.uv[0], mesh.uv[1]);

    pbr_input.material.base_color = textureSample(my_array_texture, my_array_texture_sampler, mesh.uv, mesh.texture_index);
#ifdef VERTEX_COLORS
    pbr_input.material.base_color = pbr_input.material.base_color * mesh.color;
#endif

    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        is_front,
    );

    pbr_input.is_orthographic = view.projection[3].w == 1.0;

    pbr_input.N = fns::apply_normal_mapping(
        pbr_input.material.flags,
        mesh.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        mesh.world_tangent,
#endif
#endif
        mesh.uv,
        view.mip_bias,
    );
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::pbr(pbr_input), view.color_grading);
}