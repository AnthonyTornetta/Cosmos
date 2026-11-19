import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import package::cosmos::shaders::accretion_disk::sample_disk;

struct LensSettings {
    screen: vec4<f32>,
    depth: vec4<f32>,
    viewport: vec4<f32>,
    disk: vec4<f32>,
}

@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var scene_depth: texture_depth_2d;
@group(0) @binding(3) var<uniform> lens: LensSettings;

fn far_side_image(radial: vec2<f32>) -> vec4<f32> {
    let up = lens.disk.xy;
    let side = vec2<f32>(-up.y, up.x);
    let local = vec2<f32>(dot(radial, side), dot(radial, up));
    let radius = length(local);
    if local.y <= 0.0 || radius <= 1.025 || radius >= 1.8 {
        return vec4<f32>(0.0);
    }
    // Compress the far half of the same disk into an arc above the shadow.
    // The front disk is drawn afterward, naturally covering the arc's base.
    let radial_uv = (radius - 1.025) / (1.8 - 1.025);
    let hemisphere = select(-1.0, 1.0, lens.disk.w >= 0.0);
    let angle = lens.disk.z + atan2(local.x, local.y) * hemisphere;
    let gas = sample_disk(radial_uv, angle, lens.depth.w, 0.1);
    let visibility = smoothstep(0.0, 0.08, local.y)
        * (1.0 - smoothstep(0.65, 0.95, abs(lens.disk.w)));
    return vec4<f32>(gas.rgb * 0.7, gas.a) * visibility;
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let original = textureSampleLevel(scene, scene_sampler, in.uv, 0.0);
    if any(in.uv < lens.viewport.xy) || any(in.uv > lens.viewport.zw) {
        return original;
    }
    let dimensions = vec2<f32>(textureDimensions(scene));
    let radial = (in.uv - lens.screen.xy) * vec2<f32>(dimensions.x / dimensions.y, 1.0);
    let impact = length(radial) / max(lens.screen.z, 0.000001);
    // The direct disk has not been drawn yet. Only background light is warped.
    let destination_depth = textureLoad(scene_depth, vec2<i32>(in.position.xy), 0);
    if destination_depth > lens.depth.x {
        return original;
    }
    let band = smoothstep(1.12, 1.75, impact) * (1.0 - smoothstep(1.75, 2.8, impact));
    // Bound the displacement in horizon radii. A monotonic mapping avoids
    // stretching a second dark image of the shadow into the surrounding gas.
    let scale = 1.0 - lens.screen.w * band / max(impact, 1.0);
    let half_pixel = 0.5 / dimensions;
    let source = clamp(lens.screen.xy + (in.uv - lens.screen.xy) * scale,
        lens.viewport.xy + half_pixel, lens.viewport.zw - half_pixel);

    // Reverse-Z depth stays in floating point even millions of units away.
    // Check both ends of the sample to avoid warping foreground objects or
    // pulling a second copy of them into the surrounding background.
    let source_depth = textureLoad(scene_depth, vec2<i32>(source * dimensions), 0);
    var background = original;
    if band > 0.0 && source_depth <= lens.depth.x {
        background = textureSampleLevel(scene, scene_sampler, source, 0.0);
    }
    let secondary = far_side_image(radial / max(lens.screen.z, 0.000001));
    return vec4<f32>(secondary.rgb + background.rgb * (1.0 - secondary.a), background.a);
}
