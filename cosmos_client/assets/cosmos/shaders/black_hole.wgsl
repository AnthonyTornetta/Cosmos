#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::{globals, view},
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> effect: vec4<f32>;

const TAU: f32 = 6.28318530718;

fn accretion_disk(in: VertexOutput) -> vec4<f32> {
    let radial = clamp(in.uv.x, 0.0, 1.0);
    let angle = in.uv.y * TAU;
    let time = globals.time;

    let warped_angle = angle
        + 0.16 * sin(angle * 3.0 - time * 0.24)
        + 0.06 * sin(angle * 11.0 + radial * 17.0);
    let broad_streak = 0.5 + 0.5 * sin(warped_angle * 15.0 - time * 1.7 + radial * 42.0);
    let fine_streak = 0.5 + 0.5 * sin(warped_angle * 37.0 - time * 3.1 - radial * 83.0);
    let turbulence = mix(broad_streak, fine_streak, 0.28);

    let inner_fade = smoothstep(0.0, 0.055, radial);
    let outer_fade = 1.0 - smoothstep(0.64, 1.0, radial);
    let bands = 0.60 + 0.40 * sin(radial * 108.0 + broad_streak * 2.2);
    let density = inner_fade * outer_fade * mix(0.34, 1.0, turbulence) * mix(0.62, 1.0, bands);

    let heat = 1.0 - radial;
    let outer_color = vec3<f32>(0.42, 0.012, 0.001);
    let middle_color = vec3<f32>(4.20, 0.42, 0.018);
    let inner_color = vec3<f32>(10.00, 4.80, 1.15);
    var color = mix(outer_color, middle_color, smoothstep(0.05, 0.72, heat));
    color = mix(color, inner_color, smoothstep(0.70, 1.0, heat));

    // Approximate relativistic beaming: one side of the rotating disk is
    // brighter and slightly hotter than the receding side.
    let approaching = 0.42 + 0.92 * smoothstep(-1.0, 1.0, sin(angle));
    let alpha = density * (0.52 + 0.48 * heat);
    return vec4<f32>(color * approaching * alpha * effect.y, alpha);
}

fn lensed_disk(in: VertexOutput) -> vec4<f32> {
    let to_camera = normalize(view.world_position - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let facing = abs(dot(normal, to_camera));
    let rim = 1.0 - facing;
    let rim2 = rim * rim;
    let rim4 = rim2 * rim2;
    // Bands above and below the disk approximate the far-side image bent over
    // the event horizon in the NASA visualization.
    let latitude = abs(normal.y);
    let upper_and_lower = 1.0 - smoothstep(0.10, 0.28, abs(latitude - 0.30));
    let equatorial_fold = 1.0 - smoothstep(0.02, 0.16, latitude);
    let arcs = max(upper_and_lower * rim2, equatorial_fold * rim4 * 0.65);
    let side_axis = cross(vec3<f32>(0.0, 1.0, 0.0), to_camera);
    let side = dot(side_axis / max(length(side_axis), 0.001), normal);
    let approaching = 0.40 + 0.95 * smoothstep(-1.0, 1.0, side);
    let flicker = 0.86 + 0.14 * sin(globals.time * 1.4 + in.uv.x * TAU * 9.0);
    let alpha = arcs * approaching * flicker * 0.74;
    let color = mix(vec3<f32>(2.8, 0.16, 0.006), vec3<f32>(9.0, 4.1, 0.85), rim);
    return vec4<f32>(color * alpha * effect.y, alpha);
}

fn photon_ring(in: VertexOutput) -> vec4<f32> {
    let to_camera = normalize(view.world_position - in.world_position.xyz);
    let normal = normalize(in.world_normal);
    let rim = 1.0 - abs(dot(normal, to_camera));
    let rim2 = rim * rim;
    let rim4 = rim2 * rim2;
    let rim8 = rim4 * rim4;
    let side_axis = cross(vec3<f32>(0.0, 1.0, 0.0), to_camera);
    let side = dot(side_axis / max(length(side_axis), 0.001), normal);
    let approaching = 0.48 + 0.72 * smoothstep(-1.0, 1.0, side);
    let flicker = 0.88 + 0.12 * sin(in.uv.x * TAU * 13.0 - globals.time * 2.1);
    let alpha = clamp(rim8 * approaching * flicker, 0.0, 1.0);
    let color = mix(vec3<f32>(3.2, 0.30, 0.015), vec3<f32>(11.0, 6.0, 1.8), approaching);
    return vec4<f32>(color * alpha * effect.y, alpha);
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    if effect.x < 0.5 {
        out.color = accretion_disk(in);
    } else if effect.x < 1.5 {
        out.color = lensed_disk(in);
    } else {
        out.color = photon_ring(in);
    }
    return out;
}
