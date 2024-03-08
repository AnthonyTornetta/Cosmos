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

// fn hash(value: u32) -> u32 {
//     var state = value;
//     state = state ^ 2747636419u;
//     state = state * 2654435769u;
//     state = state ^ state >> 16u;
//     state = state * 2654435769u;
//     state = state ^ state >> 16u;
//     state = state * 2654435769u;
//     return state;
// }

// fn randomFloat(value: u32) -> f32 {
//     return f32(hash(value)) / 4294967295.0;
// }

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

//     let temperature = noise.get([
//         self.temperature_seed.0 + lx,
//         self.temperature_seed.1 + ly,
//         self.temperature_seed.2 + lz,
//     ]);

//     let humidity = noise.get([self.humidity_seed.0 + lx, self.humidity_seed.1 + ly, self.humidity_seed.2 + lz]);

//     let elevation = noise.get([self.elevation_seed.0 + lx, self.elevation_seed.1 + ly, self.elevation_seed.2 + lz]);

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

fn calculate_value_at(coords_f32: vec3<f32>) -> i32 {
    var iterations = 9;
    let delta = f64(0.01);

    // let amplitude = noise(
    //         f64(coords_f32.x + 1.0) * (delta),
    //         f64(coords_f32.y - 1.0) * (delta),
    //         f64(coords_f32.z + 1.0) * (delta),
    //     ) * 9.0;

    let amplitude = f64(4.0);

    var depth: f64 = 0.0;

    while iterations > 0 {
        let iteration = f64(iterations);

        depth += noise(
            f64(coords_f32.x) * (delta / f64(iteration)),
            f64(coords_f32.y) * (delta / f64(iteration)),
            f64(coords_f32.z) * (delta / f64(iteration)),
        ) * amplitude * iteration;

        iterations -= 1;
    }

    var coord: f32 = coords_f32.x;
    
    let face = planet_face_relative(vec3(coords_f32.x, coords_f32.y, coords_f32.z));

    if face == BF_TOP || face == BF_BOTTOM {
        coord = coords_f32.y;
    }
    else if face == BF_FRONT || face == BF_BACK {
        coord = coords_f32.z;
    }

    let depth_here = f32(params.sea_level.y) + f32(depth);

    let block_depth = i32(floor(depth_here - abs(coord)));

    return block_depth;
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

    // var iterations = 9;
    // let delta = f64(0.01);

    // // let amplitude = noise(
    // //         f64(coords_f32.x + 1.0) * (delta),
    // //         f64(coords_f32.y - 1.0) * (delta),
    // //         f64(coords_f32.z + 1.0) * (delta),
    // //     ) * 9.0;

    // let amplitude = f64(4.0);

    // var depth: f64 = 0.0;

    // while iterations > 0 {
    //     let iteration = f64(iterations);

    //     depth += noise(
    //         f64(coords_f32.x) * (delta / f64(iteration)),
    //         f64(coords_f32.y) * (delta / f64(iteration)),
    //         f64(coords_f32.z) * (delta / f64(iteration)),
    //     ) * amplitude * iteration;

    //     iterations -= 1;
    // }

    // // 9.0 * f32(noise(f64(coords_f32.x * 0.01), f64(coords_f32.y * 0.01), f64(coords_f32.z * 0.01)))

    // // let has_block = abs(coords_f32.y) < f32(params.sea_level.y) + sin(0.1 * (coords_f32.x + coords_f32.z)) * 9;
    // var coord: f32 = coords_f32.x;
    
    // let face = planet_face_relative(vec3(coords_f32.x, coords_f32.y, coords_f32.z));

    // if face == BF_TOP || face == BF_BOTTOM {
    //     coord = coords_f32.y;
    // }
    // else if face == BF_FRONT || face == BF_BACK {
    //     coord = coords_f32.z;
    // }

    // let depth_here = f32(params.sea_level.y) + f32(depth);

    // let has_block = depth_here - abs(coord);

    let coords_vec3 = vec3(coords_f32.x, coords_f32.y, coords_f32.z);

    var depth_here = calculate_value_at(coords_vec3);

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

        if calculate_value_at(coords_vec3 + delta) < 0 {
            depth_here = 0;
        }
    }

    // values[idx] = f32(params.sea_level);// f32(coords_f32.y < params.sea_level);//f32(coords_f32.y < 500.0);// params.chunk_coords.x + params.structure_pos.x;
    values[idx] = f32(depth_here);// f32(coords_f32.y < params.sea_level.y); //f32(coords_f32.y < 500.0);// params.chunk_coords.x + params.structure_pos.x;
    // values[idx] = 1.0 + params.chunk_coords.x;
}











/**
 * Helper function to hash an integer using the above permutation table
 *
 *  This inline function costs around 1ns, and is called N+1 times for a noise of N dimension.
 *
 *  Using a real hash function would be better to improve the "repeatability of 256" of the above permutation table,
 * but fast integer Hash functions uses more time and have bad random properties.
 *
 * @param[in] i Integer value to hash
 *
 * @return 8-bits hashed value
 */
fn hash(i: u32) -> i32 {
    var perm: array<i32, 256> = array(
        151,160,137,91,90,15,131,13,201,95,96,53,194,233,7,225,140,36,103,30,69,142,8,99,37,240,21,10,23,190,6,
        148,247,120,234,75,0,26,197,62,94,252,219,203,117,35,11,32,57,177,33,88,237,149,56,87,174,20,125,136,171,
        168,68,175,74,165,71,134,139,48,27,166,77,146,158,231,83,111,229,122,60,211,133,230,220,105,92,41,55,46,
        245,40,244,102,143,54,65,25,63,161,1,216,80,73,209,76,132,187,208,89,18,169,200,196,135,130,116,188,159,
        86,164,100,109,198,173,186,3,64,52,217,226,250,124,123,5,202,38,147,118,126,255,82,85,212,207,206,59,227,
        47,16,58,17,182,189,28,42,223,183,170,213,119,248,152,2,44,154,163,70,221,153,101,155,167,43,172,9,129,22,
        39,253,19,98,108,110,79,113,224,232,178,185,112,104,218,246,97,228,251,34,242,193,238,210,144,12,191,179,
        162,241,81,51,145,235,249,14,239,107,49,192,214,31,181,199,106,157,184,84,204,176,115,121,50,45,127,4,150,
        254,138,236,205,93,222,114,67,29,24,72,243,141,128,195,78,66,215,61,156,180
    );
    return perm[i];
}

/**
 * Helper functions to compute gradients-dot-residual vectors (3D)
 *
 * @param[in] hash  hash value
 * @param[in] x     x coord of the distance to the corner
 * @param[in] y     y coord of the distance to the corner
 * @param[in] z     z coord of the distance to the corner
 *
 * @return gradient value
 */
fn grad(hash: u32, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let hl8 = f64(h < 8); // Convert low 4 bits of hash code into 12 simple
    let u = (hl8 * x) + ((1.0 - hl8) * y); // gradient directions, and compute dot product.
    let hl4 = f64(h < 4);
    let otr = f64(h == 12 || h == 14);
    let v = (hl4 * y) + (1.0 - hl4) * (otr * x) + (1.0 - otr) * z; // Fix repeats at h = 12 to 15

    let hand1 = f64(h & 1);
    let hand2 = f64(h & 2);

    return (hand1 * -1.0 + (1.0 - hand1)) * u + (hand2 * -1.0 + (1.0 - hand2)) * v;
}


// Skewing/Unskewing factors for 3D
const F3 = f64(1.0 / 3.0);
const G3 = f64(1.0 / 6.0);

/**
 * Translated from: https://github.com/SRombauts/SimplexNoise/blob/master/src/SimplexNoise.cpp
 * 
 * 3D Perlin simplex noise
 *
 * @param[in] x float coordinate
 * @param[in] y float coordinate
 * @param[in] z float coordinate
 *
 * @return Noise value in the range[-1; 1], value of 0 on all integer coordinates.
 */
fn noise(x: f64, y: f64, z: f64) -> f64 {
    var n0 = f64(0.0);
    var n1 = f64(0.0);
    var n2 = f64(0.0);
    var n3 = f64(0.0); // Noise contributions from the four corners

    // Skew the input space to determine which simplex cell we're in
    let s = (x + y + z) * F3; // Very nice and simple skew factor for 3D
    let i = u32(floor(x + s));
    let j = u32(floor(y + s));
    let k = u32(floor(z + s));
    let t = f64((i + j + k)) * G3;
    let X0 = f64(i) - t; // Unskew the cell origin back to (x,y,z) space
    let Y0 = f64(j) - t;
    let Z0 = f64(k) - t;
    let x0 = x - X0; // The x,y,z distances from the cell origin
    let y0 = y - Y0;
    let z0 = z - Z0;

    // For the 3D case, the simplex shape is a slightly irregular tetrahedron.
    // Determine which simplex we are in.
    var i1 = 0;
    var j1 = 0;
    var k1 = 0; // Offsets for second corner of simplex in (i,j,k) coords
    var i2 = 0;
    var j2 = 0;
    var k2 = 0; // Offsets for third corner of simplex in (i,j,k) coords
    if x0 >= y0 {
        if y0 >= z0 {
            i1 = 1;
            j1 = 0;
            k1 = 0;
            i2 = 1;
            j2 = 1;
            k2 = 0; // X Y Z order
        } else if x0 >= z0 {
            i1 = 1;
            j1 = 0;
            k1 = 0;
            i2 = 1;
            j2 = 0;
            k2 = 1; // X Z Y order
        } else {
            i1 = 0;
            j1 = 0;
            k1 = 1;
            i2 = 1;
            j2 = 0;
            k2 = 1; // Z X Y order
        }
    } else {
        // x0<y0
        if y0 < z0 {
            i1 = 0;
            j1 = 0;
            k1 = 1;
            i2 = 0;
            j2 = 1;
            k2 = 1; // Z Y X order
        } else if x0 < z0 {
            i1 = 0;
            j1 = 1;
            k1 = 0;
            i2 = 0;
            j2 = 1;
            k2 = 1; // Y Z X order
        } else {
            i1 = 0;
            j1 = 1;
            k1 = 0;
            i2 = 1;
            j2 = 1;
            k2 = 0; // Y X Z order
        }
    }

    // A step of (1,0,0) in (i,j,k) means a step of (1-c,-c,-c) in (x,y,z),
    // a step of (0,1,0) in (i,j,k) means a step of (-c,1-c,-c) in (x,y,z), and
    // a step of (0,0,1) in (i,j,k) means a step of (-c,-c,1-c) in (x,y,z), where
    // c = 1/6.
    let x1 = x0 - f64(i1) + G3; // Offsets for second corner in (x,y,z) coords
    let y1 = y0 - f64(j1) + G3;
    let z1 = z0 - f64(k1) + G3;
    let x2 = x0 - f64(i2) + 2.0 * G3; // Offsets for third corner in (x,y,z) coords
    let y2 = y0 - f64(j2) + 2.0 * G3;
    let z2 = z0 - f64(k2) + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3; // Offsets for last corner in (x,y,z) coords
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    // Work out the hashed gradient indices of the four simplex corners
    let gi0 = hash(u32(i) + u32(hash(u32(j) + u32(hash(k)))));
    let gi1 = hash(u32(i) + u32(i1) + u32(hash(u32(j) + u32(j1) + u32(hash(u32(k) + u32(k1))))));
    let gi2 = hash(u32(i) + u32(i2) + u32(hash(u32(j) + u32(j2) + u32(hash(u32(k) + u32(k2))))));
    let gi3 = hash(u32(i) + u32(1) + u32(hash(u32(j) + u32(1) + u32(hash(u32(k) + u32(1))))));

    // Calculate the contribution from the four corners
    var t0 = f64(0.6 - x0 * x0 - y0 * y0 - z0 * z0);
    if t0 < 0.0 {
        n0 = f64(0.0);
    } else {
        t0 *= t0;
        n0 = t0 * t0 * grad(u32(gi0), x0, y0, z0);
    }
    var t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 < 0.0 {
        n1 = f64(0.0);
    } else {
        t1 *= t1;
        n1 = t1 * t1 * grad(u32(gi1), x1, y1, z1);
    }
    var t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 < 0.0 {
        n2 = f64(0.0);
    } else {
        t2 *= t2;
        n2 = t2 * t2 * grad(u32(gi2), x2, y2, z2);
    }
    var t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 < 0.0 {
        n3 = f64(0.0);
    } else {
        t3 *= t3;
        n3 = t3 * t3 * grad(u32(gi3), x3, y3, z3);
    }
    // Add contributions from each corner to get the final noise value.
    // The result is scaled to stay just inside [-1,1]
    return 32.0 * (n0 + n1 + n2 + n3);
}
