#import "cosmos/shaders/biosphere/generation_utils.wgsl"::{
    GenerationParams,
    TerrainData,
    BF_BACK, BF_FRONT, BF_LEFT, BF_RIGHT, BF_TOP, BF_BOTTOM,
    planet_face_relative,
    calculate_depth_at,
    calculate_biome_parameters,
};

fn generate(
    param: GenerationParams,
    coords: vec4u
) -> TerrainData {
    let coords_f32: vec4<f32> = vec4(f32(coords.x), f32(coords.y), f32(coords.z), 0.0) * param.scale + param.chunk_coords;
    let coords_vec3 = vec3(coords_f32.x, coords_f32.y, coords_f32.z);
    let sea_level = param.sea_level.y;

    var depth_here = calculate_depth_at(coords_vec3, sea_level);

    if depth_here >= 0 && depth_here < 10 {
        let face = planet_face_relative(coords_vec3);
        var delta: vec3<f32> = vec3(0, 0, 0);

        switch face {
            case BF_TOP: {
                delta = vec3(0.0, 1.0, 0.0);
                break;
            }
            case BF_BOTTOM: {
                delta = vec3(0.0, -1.0, 0.0);
                break;
            }
            case BF_RIGHT: {
                delta = vec3(1.0, 0.0, 0.0);
                break;
            }
            case BF_LEFT: {
                delta = vec3(-1.0, 0.0, 0.0);
                break;
            }
            case BF_FRONT: {
                delta = vec3(0.0, 0.0, 1.0);
                break;
            }
            case BF_BACK: {
                delta = vec3(0.0, 0.0, -1.0);
                break;
            }
            default:
            {
                // This will never happen
                break;
            }
        }

        let scale_f32 = vec3(param.scale.x, param.scale.y, param.scale.z);

        let value_above = calculate_depth_at(coords_vec3 + delta * scale_f32, sea_level);
        if value_above < 0 {
            // There is no block above us, so make sure we're the top layer.
            depth_here = 0;
        } else if depth_here == 0 && value_above >= 0 {
            // There is a block above us, so ensure we're not the top layer.
            depth_here = 1;
        }
    }

    let biome_data = calculate_biome_parameters(coords_f32, param.structure_pos);

    return TerrainData(depth_here, biome_data);
}
