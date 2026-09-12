#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

struct BlackHoleLensSettings {
    screen: vec4<f32>,
    depth: vec4<f32>,
}

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;
@group(0) @binding(2) var scene_depth_texture: texture_depth_2d;
@group(0) @binding(3) var<uniform> settings: BlackHoleLensSettings;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(screen_texture));
    let aspect = dimensions.x / dimensions.y;
    let radial = vec2<f32>(
        (in.uv.x - settings.screen.x) * aspect,
        in.uv.y - settings.screen.y,
    );
    let impact = length(radial) / max(settings.screen.z, 0.000001);

    // Preserve the center of the shadow. The bending rises immediately
    // outside the event horizon, peaks around the photon ring, and fades
    // seamlessly into the undistorted scene.
    let inner_fade = smoothstep(0.54, 0.64, impact);
    let outer_fade = 1.0 - smoothstep(0.86, 1.18, impact);
    let lens_band = inner_fade * outer_fade;

    // Keep the source coordinate outside the shadow. This stretches the
    // complete rendered image around the photon ring without pulling a dark
    // copy of the event horizon into the surrounding lens field.
    let desired_scale = 1.0 - settings.screen.w * lens_band;
    let horizon_safe_scale = 0.575 / max(impact, 0.575);
    let sample_scale = max(desired_scale, horizon_safe_scale);
    let source_uv = clamp(
        settings.screen.xy + (in.uv - settings.screen.xy) * sample_scale,
        vec2<f32>(0.001),
        vec2<f32>(0.999),
    );

    // Bevy uses reverse-Z: larger values are closer. Preserve foreground
    // geometry at both the destination and source positions, preventing the
    // lens from warping it or smearing it into adjacent background pixels.
    let destination_pixel = vec2<i32>(in.position.xy);
    let source_pixel = vec2<i32>(source_uv * dimensions);
    let destination_depth = textureLoad(scene_depth_texture, destination_pixel, 0);
    let source_depth = textureLoad(scene_depth_texture, source_pixel, 0);
    if max(destination_depth, source_depth) > settings.depth.x {
        return textureSample(screen_texture, screen_sampler, in.uv);
    }

    return textureSample(screen_texture, screen_sampler, source_uv);
}
