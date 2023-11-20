#import bevy_pbr::forward_io::VertexOutput

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
var<uniform> color: vec3<f32>;

fn get_texture_sample(coords: vec2<f32>) -> vec4<f32> {
    let repeated_coords = vec2<f32>(
        (coords.x % (1.0 / f32(repeats.horizontal))) * f32(repeats.horizontal),
        (coords.y % (1.0 / f32(repeats.vertical))) * f32(repeats.vertical)
    );
    return textureSample(texture, texture_sampler, repeated_coords);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var texture = get_texture_sample(in.uv);
    texture *= vec4(color, 1.0);

    if texture[3] < 0.5 {
        discard;
    }

    return texture;
}