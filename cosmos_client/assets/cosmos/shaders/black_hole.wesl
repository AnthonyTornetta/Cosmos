import bevy_pbr::{
    render::forward_io::{VertexOutput, FragmentOutput},
    render::mesh_view_bindings::globals,
};
import package::cosmos::shaders::accretion_disk::sample_disk;

@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> intensity: f32;

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    // Annulus UVs run radially along x and around the disk along y.
    let radial = clamp(in.uv.x, 0.0, 1.0);
    let angle = in.uv.y * 6.28318530718;
    let softness = min((fwidth(radial) * 25.1327412287 + fwidth(angle) * 2.0) * 0.3, 0.2);
    let gas = sample_disk(radial, angle, globals.time, softness);
    var out: FragmentOutput;
    out.color = vec4<f32>(gas.rgb * intensity, gas.a);
    return out;
}
