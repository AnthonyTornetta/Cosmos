import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_view_bindings::view,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::main_pass_post_lighting_processing,
};
import super::planet_sky_model::{SkyParameters, sky_color};

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var<uniform> sky: SkyParameters;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let direction = normalize(in.world_position.xyz - view.world_position);
    let color = sky_color(direction, sky);
    let pbr_input = pbr_input_from_standard_material(in, is_front);
    var out: FragmentOutput;
    out.color = main_pass_post_lighting_processing(pbr_input, color);
    return out;
}
