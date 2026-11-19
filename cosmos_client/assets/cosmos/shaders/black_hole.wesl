import bevy_pbr::{
    render::forward_io::{VertexOutput, FragmentOutput},
    render::mesh_view_bindings::globals,
};

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> intensity: f32;

const TAU: f32 = 6.28318530718;
const INNER_ANGULAR_SPEED: f32 = 0.9;

fn disk_hash(cell: vec2<f32>, angular_period: f32) -> f32 {
    // Wrap the angular lattice, including negative angles after rotation.
    let wrapped = vec2<f32>(cell.x - floor(cell.x / angular_period) * angular_period, cell.y);
    return fract(sin(dot(wrapped, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn disk_noise(point: vec2<f32>, angular_period: f32) -> f32 {
    let cell = floor(point);
    let fraction = fract(point);
    let blend = fraction * fraction * (3.0 - 2.0 * fraction);
    let lower = mix(disk_hash(cell, angular_period), disk_hash(cell + vec2<f32>(1.0, 0.0), angular_period), blend.x);
    let upper = mix(disk_hash(cell + vec2<f32>(0.0, 1.0), angular_period), disk_hash(cell + vec2<f32>(1.0, 1.0), angular_period), blend.x);
    return mix(lower, upper, blend.y);
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    // Annulus UVs run radially along x and around the disk along y.
    let radial = clamp(in.uv.x, 0.0, 1.0);
    let angle = in.uv.y * TAU;

    // Advect the pattern at each radius: inner gas overtakes the outer gas.
    // Use radii relative to the horizon so motion is independent of world scale.
    let orbit_radius = mix(1.12, 4.2, radial);
    let angular_speed = INNER_ANGULAR_SPEED * pow(1.12 / orbit_radius, 1.5);
    let orbit_angle = angle - globals.time * angular_speed;

    // A continuous turbulent body, with winding lanes cutting through the gas.
    let turbulence = disk_noise(vec2<f32>(orbit_angle / TAU * 12.0, radial * 18.0), 12.0);
    let detail = disk_noise(vec2<f32>(orbit_angle / TAU * 24.0 + 2.7, radial * 42.0), 24.0);
    let phase = radial * TAU * 4.0 + orbit_angle * 2.0
        + sin(orbit_angle * 3.0 + radial * 10.0) + (turbulence - 0.5) * 3.5;
    let lane = 0.5 + 0.5 * sin(phase);
    let edge_softness = min(fwidth(phase) * 0.3, 0.2);
    let ribbon = smoothstep(0.2 - edge_softness, 0.8 + edge_softness, lane);
    let patches = smoothstep(0.2, 0.8, turbulence * 0.7 + detail * 0.3);
    // Broad moving highlights make the orbit readable through the bloom.
    let stream = 0.96 + 0.24 * sin(orbit_angle * 2.0 + radial * 4.0);
    let density = mix(0.65, 1.25, patches) * mix(0.5, 1.25, ribbon) * stream;

    let outer_edge = radial + (0.5 - turbulence) * 0.035;
    let fade = smoothstep(0.0, 0.035, radial) * (1.0 - smoothstep(0.65, 1.0, outer_edge));
    let heat = (1.0 - radial) * (1.0 - radial);
    let color = mix(vec3<f32>(2.1, 0.22, 0.015), vec3<f32>(8.0, 4.2, 1.2), heat);
    let opacity = fade * mix(0.8, 0.99, patches);

    var out: FragmentOutput;
    // Dense gas obscures background stars while its HDR emission still blooms.
    out.color = vec4<f32>(color * density * intensity * opacity, opacity);
    return out;
}
