#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
    mesh_view_bindings::globals,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(2) @binding(100) var<uniform> ripples: array<vec4<f32>, 20>;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    // in deferred mode we can't modify anything after that, as lighting is run in a separate fullscreen shader.
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // apply lighting
    var color = apply_pbr_lighting(pbr_input);
    out.color = vec4(0.0, 0.0, 0.0, 0.0);

    let dotted = dot(
        normalize(vec3(ripples[0].x, ripples[0].y, ripples[0].z)), 
        normalize(vec3(in.world_normal.x, in.world_normal.y, in.world_normal.z))
    );

    let c = 1000.0;
    let e = 2.718;

    let exponential = 8.0 * pow(e, c * (-1 + dotted));

    if dotted >= 0.0 {
        out.color += color * vec4(exponential, exponential, exponential, exponential);
    }

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

    // we can optionally modify the final result here
    // out.color = out.color * 2.0;
#endif


    return out;
}