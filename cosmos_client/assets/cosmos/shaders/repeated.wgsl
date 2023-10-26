// Stolen and heavily modified from: https://github.com/janhohenheim/foxtrot/blob/main/assets/shaders/repeated.wgsl - TY!

#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_vertex_output as OutputTypes
#import bevy_pbr::pbr_functions as PbrCore
#import bevy_pbr::pbr_bindings as MaterialBindings
#import bevy_pbr::pbr_types as PbrTypes
#import bevy_pbr::mesh_view_bindings as ViewBindings

struct Repeats {
    horizontal: u32,
    vertical: u32,
    _wasm_padding1: u32,
    _wasm_padding2: u32,
}

@group(1) @binding(0)
var texture: texture_2d<f32>;
@group(1) @binding(1)
var texture_sampler: sampler;
@group(1) @binding(2)
var<uniform> repeats: Repeats;
@group(1) @binding(3)
var<uniform> color: vec4<f32>;

fn get_texture_sample(coords: vec2<f32>) -> vec4<f32> {
    let repeated_coords = vec2<f32>(
        (coords.x % (1. / f32(repeats.horizontal))) * f32(repeats.horizontal),
        (coords.y % (1. / f32(repeats.vertical))) * f32(repeats.vertical)
    );
    return textureSample(texture, texture_sampler, repeated_coords);
}

/// Adapted from <https://github.com/bevyengine/bevy/blob/main/crates/bevy_pbr/src/render/pbr.wgsl#L30>
fn get_pbr_output(in: MeshVertexOutput) -> vec4<f32> {
    var material = PbrTypes::standard_material_new();
    material.perceptual_roughness = 1.0;

    var output_color: vec4<f32> = color;
    
    output_color = PbrCore::alpha_discard(material, output_color);

#ifdef TONEMAP_IN_SHADER
        output_color = tone_mapping(output_color);
#endif
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.frag_coord.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#ifdef PREMULTIPLY_ALPHA
        output_color = premultiply_alpha(material.flags, output_color);
#endif
    return output_color;
}

@fragment
fn fragment(mesh: MeshVertexOutput) -> @location(0) vec4<f32> {
    let texture = get_texture_sample(mesh.uv);
    if (texture[3] < 0.5) {
        discard;
    }
    let pbr_output = get_pbr_output(mesh);

    return texture * pbr_output;
}