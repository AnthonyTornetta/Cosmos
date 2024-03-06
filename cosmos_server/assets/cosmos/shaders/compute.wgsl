// @group(0) @binding(0) var texture: texture_storage_2d<r32float, read_write>;
@group(0) @binding(0) var<uniform> params: GenerationParams;
@group(0) @binding(1) var<storage, read_write> values: array<f32>;

struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    chunk_coords: vec4<f32>,
    structure_pos: vec4<f32>,
    sea_level: vec4<f32>,
    scale: vec4<f32>,
}

// enum BlockFace {
//     Back, Bottom, Left, Front, Top, Right
// }

const BF_BACK: i32 = 1;
const BF_FRONT: i32 = 2;
const BF_LEFT: i32 = 3;
const BF_RIGHT: i32 = 4;
const BF_BOTTOM: i32 = 5;
const BF_TOP: i32 = 6;

/// Gets the face of a planet this location is closest to. Prioritizes negative sides to make positive-to-negative edges look ok.
fn planet_face_relative(relative_position: vec3<f32>) -> i32 {
    let normalized = normalize(relative_position);
    let abs = abs(normalized);

    let max = max(abs.x, max(abs.y, abs.z));

    if normalized.z < 0 && abs.z == max {
        return BF_BACK;
    } else if normalized.y < 0 && abs.y == max {
        return BF_BOTTOM;
    } else if normalized.x < 0 && abs.x == max {
        return BF_LEFT;
    } else if abs.z == max {
        return BF_FRONT;
    } else if abs.y == max {
        return BF_TOP;
    } else {
        return BF_RIGHT;
    }
}

fn hash(value: u32) -> u32 {
    var state = value;
    state = state ^ 2747636419u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    state = state ^ state >> 16u;
    state = state * 2654435769u;
    return state;
}

fn randomFloat(value: u32) -> f32 {
    return f32(hash(value)) / 4294967295.0;
}

// @compute @workgroup_size(8, 8, 1)
// fn init(@builtin(global_invocation_id) invocation_id: vec3<u32>, @builtin(num_workgroups) num_workgroups: vec3<u32>) {
//     // let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

//     // let randomNumber = randomFloat(invocation_id.y * num_workgroups.x + invocation_id.x);
//     // let alive = randomNumber > 0.9;
//     // let color = vec4<f32>(f32(alive));

//     // textureStore(texture, location, color);
// }

// fn is_alive(location: vec2<i32>, offset_x: i32, offset_y: i32) -> i32 {
//     let value: vec4<f32> = textureLoad(texture, location + vec2<i32>(offset_x, offset_y));
//     return i32(value.x);
// }

// fn count_alive(location: vec2<i32>) -> i32 {
//     return is_alive(location, -1, -1) +
//            is_alive(location, -1,  0) +
//            is_alive(location, -1,  1) +
//            is_alive(location,  0, -1) +
//            is_alive(location,  0,  1) +
//            is_alive(location,  1, -1) +
//            is_alive(location,  1,  0) +
//            is_alive(location,  1,  1);
// }

// const BIOME_DECIDER_DELTA: f64 = 0.01;

// fn decide_biome(sx: f64, sy: f64, sz: f64, bx: f32, by: f32, bz: f32) {
//     let (lx, ly, lz) = (
//         (sx + f64(bx)) * BIOME_DECIDER_DELTA,
//         (sy + f64(by)) * BIOME_DECIDER_DELTA,
//         (sz + f64(bz)) * BIOME_DECIDER_DELTA,
//     );

//     let mut temperature = noise.get([
//         self.temperature_seed.0 + lx,
//         self.temperature_seed.1 + ly,
//         self.temperature_seed.2 + lz,
//     ]);

//     let mut humidity = noise.get([self.humidity_seed.0 + lx, self.humidity_seed.1 + ly, self.humidity_seed.2 + lz]);

//     let mut elevation = noise.get([self.elevation_seed.0 + lx, self.elevation_seed.1 + ly, self.elevation_seed.2 + lz]);

//     // Clamps all values to be [0, 100.0)

//     temperature = (temperature.min(0.999).max(-1.0) * 0.5 + 0.5) * 100.0;
//     humidity = (humidity.min(0.999).max(-1.0) * 0.5 + 0.5) * 100.0;
//     elevation = (elevation.min(0.999).max(-1.0) * 0.5 + 0.5) * 100.0;

//     debug_assert!((0.0..100.0).contains(&elevation), "Bad elevation: {elevation}",);
//     debug_assert!((0.0..100.0).contains(&humidity), "Bad humidity: {humidity}",);
//     debug_assert!((0.0..100.0).contains(&temperature), "Bad temperature: {temperature}",);

//     BiomeParameters {
//         ideal_elevation: elevation as f32,
//         ideal_humidity: humidity as f32,
//         ideal_temperature: temperature as f32,
//     }
// }


/// Reverses the operation of flatten, and gives the 3d x/y/z coordinates for a 3d array given a 1d array coordinate
fn expand(index: u32, width: u32, height: u32) -> vec3<u32> {
    let wh = width * height;

    let z = index / wh;
    let y = (index - z * wh) / (width);
    let x = (index - z * wh) - y * width;

    return vec3(x, y, z);
}

@compute @workgroup_size(512)
fn main(@builtin(global_invocation_id) invocation_id: vec3<u32>) {
    // let location = vec2<i32>(i32(invocation_id.x), i32(invocation_id.y));

    // let idx = invocation_id.z * 2 * 2 + invocation_id.y * 2 + invocation_id.x;
    let idx = invocation_id.x;

    // SIZE
    let coords = expand(idx, u32(32), u32(32));

    let coords_f32 = vec4(f32(coords.x), f32(coords.y), f32(coords.z), 0.0) * params.scale + params.chunk_coords;

    // let n_alive = count_alive(location);

    // var alive: bool;
    // if (n_alive == 3) {
    //     alive = true;
    // } else if (n_alive == 2) {
    //     let currently_alive = is_alive(location, 0, 0);
    //     alive = bool(currently_alive);
    // } else {
    //     alive = false;
    // }
    // let color = vec4<f32>(values[0] * alive, 0, 0, 1);

    // storageBarrier();

    // textureStore(texture, location, color);

    let has_block = abs(coords_f32.y) < f32(params.sea_level.y) + sin(0.1 * (coords_f32.x + coords_f32.z)) * 9;

    // values[idx] = f32(params.sea_level);// f32(coords_f32.y < params.sea_level);//f32(coords_f32.y < 500.0);// params.chunk_coords.x + params.structure_pos.x;
    values[idx] = f32(has_block);// f32(coords_f32.y < params.sea_level.y); //f32(coords_f32.y < 500.0);// params.chunk_coords.x + params.structure_pos.x;
    // values[idx] = 1.0 + params.chunk_coords.x;
}
