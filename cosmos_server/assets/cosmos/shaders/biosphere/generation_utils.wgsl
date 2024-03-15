@group(0) @binding(0) var<uniform> permutation_table: array<vec4<u32>, 64>;

struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    chunk_coords: vec4<f32>,
    structure_pos: vec4<f32>,
    sea_level: vec4<f32>,
    scale: vec4<f32>,
    biosphere_id: vec4<u32>,
}

struct TerrainData {
    depth: i32,
    data: u32,
}

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















/// Reverses the operation of flatten, and gives the 3d x/y/z coordinates for a 3d array given a 1d array coordinate
fn expand(index: u32, width: u32, height: u32, length: u32) -> vec4<u32> {
    let whl = width * height * length;
    let wh = width * height;

    let w = index / whl;
    let z = (index - w * whl) / wh;
    let y = ((index - w * whl) - z * wh) / width;
    let x = ((index - w * whl) - z * wh) - y * width;

    return vec4(x, y, z, w);
}

fn calculate_depth_at(coords_f32: vec3<f32>, sea_level: f32) -> i32 {
    var iterations = 9;
    let delta = f64(0.01);

    let amplitude_delta = f64(0.001);
    let amplitude = abs(noise(
            f64(coords_f32.x + 537.0) * amplitude_delta,
            f64(coords_f32.y - 1123.0) * amplitude_delta,
            f64(coords_f32.z + 1458.0) * amplitude_delta,
        )) * 9.0;

    // let amplitude = f64(9.0);

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

    let depth_here = f32(sea_level) + f32(depth);

    let block_depth = i32(floor(depth_here - abs(coord)));

    return block_depth;
}

fn calculate_biome_parameters(coords_f32: vec4<f32>, s_loc: vec4<f32>) -> u32 {
    // Random values I made up
    let elevation_seed: vec3<f64> = vec3(f64(903.0), f64(278.0), f64(510.0));
    let humidity_seed: vec3<f64> = vec3(f64(630.0), f64(238.0), f64(129.0));
    let temperature_seed: vec3<f64> = vec3(f64(410.0), f64(378.0), f64(160.0));

    let delta = f64(0.001);

    let lx = (f64(s_loc.x) + f64(coords_f32.x)) * delta;
    let ly = (f64(s_loc.y) + f64(coords_f32.y)) * delta;
    let lz = (f64(s_loc.z) + f64(coords_f32.z)) * delta;

    var temperature = noise(temperature_seed.x + lx, temperature_seed.y + ly, temperature_seed.z + lz);
    var humidity = noise(humidity_seed.x + lx, humidity_seed.y + ly, humidity_seed.z + lz);
    var elevation = noise(elevation_seed.x + lx, elevation_seed.y + ly, elevation_seed.z + lz);

    // Clamps all values to be [0, 100.0)

    temperature = (max(min(temperature, f64(0.999)), f64(-1.0)) * 0.5 + 0.5) * 100.0;
    humidity = (max(min(humidity, f64(0.999)), f64(-1.0)) * 0.5 + 0.5) * 100.0;
    elevation = (max(min(elevation, f64(0.999)), f64(-1.0)) * 0.5 + 0.5) * 100.0;

    let temperature_u32 = u32(temperature);
    let humidity_u32 = u32(humidity);
    let elevation_u32 = u32(elevation);

    // You only need 7 bits to store a number from 0 to 100, but I like << 8 better.
    return temperature_u32 << 16 | humidity_u32 << 8 | elevation_u32;
}


// Nosie functions

// Stolen from: https://github.com/Mapet13/opensimplex_noise_rust/blob/master/src/open_simplex_noise_3d.rs#L40


// STRETCH SHOULD BE NEGATIVE, but the compiler crashes whenever I make this negative. I don't know why.
// It also crashes if I try to do a divide operation here, so enjoy the long constants.
const STRETCH: f64 = 0.1666666666666666666666666666666666666666666666; // (1 / sqrt(3 + 1) - 1) / 3 == -1/6
const SQUISH: f64  = 0.3333333333333333333333333333333333333333333333; // (sqrt(3 + 1) - 1) / 3 == 1/3

const STRETCH_POINT: vec3<f64> = vec3(STRETCH, STRETCH, STRETCH);
const SQUISH_POINT: vec3<f64> = vec3(SQUISH, SQUISH, SQUISH);

const NORMALIZING_SCALAR: f64 = 103.0;

const GRAD_TABLE_LEN: u32 = 24;
const GRAD_TABLE: array<vec3<f32>, GRAD_TABLE_LEN> = array(
    vec3(f32(-11.0), f32(4.0), f32(4.0)),
    vec3(f32(-4.0), f32(11.0), f32(4.0)),
    vec3(f32(-4.0), f32(4.0), f32(11.0)),
    vec3(f32(11.0), f32(4.0), f32(4.0)),
    vec3(f32(4.0), f32(11.0), f32(4.0)),
    vec3(f32(4.0), f32(4.0), f32(11.0)),
    vec3(f32(-11.0), f32(-4.0), f32(4.0)),
    vec3(f32(-4.0), f32(-11.0), f32(4.0)),
    vec3(f32(-4.0), f32(-4.0), f32(11.0)),
    vec3(f32(11.0), f32(-4.0), f32(4.0)),
    vec3(f32(4.0), f32(-11.0), f32(4.0)),
    vec3(f32(4.0), f32(-4.0), f32(11.0)),
    vec3(f32(-11.0), f32(4.0), f32(-4.0)),
    vec3(f32(-4.0), f32(11.0), f32(-4.0)),
    vec3(f32(-4.0), f32(4.0), f32(-11.0)),
    vec3(f32(11.0), f32(4.0), f32(-4.0)),
    vec3(f32(4.0), f32(11.0), f32(-4.0)),
    vec3(f32(4.0), f32(4.0), f32(-11.0)),
    vec3(f32(-11.0), f32(-4.0), f32(-4.0)),
    vec3(f32(-4.0), f32(-11.0), f32(-4.0)),
    vec3(f32(-4.0), f32(-4.0), f32(-11.0)),
    vec3(f32(11.0), f32(-4.0), f32(-4.0)),
    vec3(f32(4.0), f32(-11.0), f32(-4.0)),
    vec3(f32(4.0), f32(-4.0), f32(-11.0)),
);

fn extrapolate(grid: vec3<f64>, delta: vec3<f64>) -> f64 {
    let point = GRAD_TABLE[get_grad_table_index(grid)];

    return f64(point.x) * delta.x + f64(point.y) * delta.y + f64(point.z) * delta.z;
}

fn noise(x: f64, y: f64, z: f64) -> f64 {
    let input: vec3<f64> = vec3(x, y, z);
    let stretch: vec3<f64> = input + ((-STRETCH_POINT) * (input.x + input.y + input.z));
    let grid = floor(stretch);

    let squashed: vec3<f64> = grid + (SQUISH_POINT * (grid.x + grid.y + grid.z));
    let ins = stretch - grid;
    let origin = input - squashed;

    // return get_value(grid, origin, ins);
    return f64(0.0);
}

fn sum(v: vec3<f64>) -> f64 {
    return v.x + v.y + v.z;
}

fn dot_self(v: vec3<f64>) -> f64 {
    return v.x * v.x + v.y * v.y + v.z * v.z;
}

fn get_grad_table_index(grid: vec3<f64>) -> u32 {
    let index0 = u32(((u32(perm((u32(grid.x) & 0xFF))) + u32(grid.y)) & 0xFF));
    let index1 = u32(((perm(index0) + u32(grid.z)) & 0xFF));
    return u32(perm(index1)) % GRAD_TABLE_LEN;
}

fn contribute(
    delta: vec3<f64>,
    origin: vec3<f64>,
    grid: vec3<f64>,
) -> f64 {
    let shifted: vec3<f64> = origin - delta - SQUISH_POINT * sum(delta);
    let attn: f64 = 2.0 - dot_self(shifted);
    if attn > 0.0 {
        return (attn*attn*attn*attn) * extrapolate(grid + delta, shifted);
    }

    return 0.0;
}

struct ClosestPoint {
    score: vec2<f64>,
    point: vec2<i32>,
}

fn determine_closest_point(
        score: vec2<f64>,
        point: vec2<i32>,
        factor: vec2<i32>,
        ins: vec3<f64>,
    ) -> ClosestPoint {
    var score_mut = score;
    var point_mut = point;
    if ins.x >= ins.y && ins.z > ins.y {
        score_mut.y = ins.z;
        point_mut.y = factor.y;
    } else if ins.x < ins.y && ins.z > ins.x {
        score_mut.x = ins.z;
        point_mut.x = factor.x;
    }

    return ClosestPoint (score_mut, point_mut);
}

fn inside_tetrahedron_at_0_0_0(
        ins: vec3<f64>,
        in_sum: f64,
        origin: vec3<f64>,
        grid: vec3<f64>,
    ) -> f64 {
    // Determine which two of (0, 0, 1), (0, 1, 0), (1, 0, 0) are closest.
    let closest_point = determine_closest_point(
        vec2(ins.x, ins.y),
        vec2(1, 2),
        vec2(4, 4),
        ins,
    );

    // Now we determine the two lattice points not part of the tetrahedron that may contribute.
    // This depends on the closest two tetrahedral vertices, including (0, 0, 0)
    let value = determine_lattice_points_including_0_0_0(
        in_sum,
        closest_point.score,
        closest_point.point,
        origin,
        grid,
    );

    return value
        + contribute(vec3(0.0, 0.0, 0.0), origin, grid)
        + contribute(vec3(1.0, 0.0, 0.0), origin, grid)
        + contribute(vec3(0.0, 1.0, 0.0), origin, grid)
        + contribute(vec3(0.0, 0.0, 1.0), origin, grid);
}

fn determine_lattice_points_including_0_0_0(
    in_sum: f64,
    score_arg: vec2<f64>,
    point: vec2<i32>,
    origin: vec3<f64>,
    grid: vec3<f64>
) -> f64 {
    let wins = 1.0 - in_sum;

    var score = score_arg;

    if wins > score.x || wins > score.y {
        // (0, 0, 0) is one of the closest two tetrahedral vertices.
        // Our other closest vertex is the closest out of a and b.
        var closest: i32;
        if score.y > score.x { 
            score = point.y; 
        } else { 
            score = point.x;
        };

        switch closest {
            case 1: {
                return contribute(vec3(1.0, -1.0, 0.0), origin, grid) + contribute(vec3(1.0, 0.0, -1.0), origin, grid);
            }
            case 2: {
                return contribute(vec3(-1.0, 1.0, 0.0), origin, grid) + contribute(vec3(0.0, 1.0, -1.0), origin, grid);
            }
            default: {
                return contribute(vec3(-1.0, 0.0, 1.0), origin, grid) + contribute(vec3(0.0, -1.0, 1.0), origin, grid); // closest == 4
            }
        }
    } else {
        // (0, 0, 0) is not one of the closest two tetrahedral vertices.
        // Our two extra vertices are determined by the closest two.
        let closest = point.x | point.y;
        switch closest {
            case 3: {
                return contribute(vec3(1.0, 1.0, 0.0), origin, grid) + contribute(vec3(1.0, 1.0, -1.0), origin, grid);
            }
            case 5: { 
                return contribute(vec3(1.0, 0.0, 1.0), origin, grid) + contribute(vec3(1.0, -1.0, 1.0), origin, grid);
            }
            default: {
                return contribute(vec3(0.0, 1.0, 1.0), origin, grid) + contribute(vec3(-1.0, 1.0, 1.0), origin, grid); // closest == 6
            }
        }
    }
}

fn get_value(grid: vec3<f64>, origin: vec3<f64>, ins: vec3<f64>) -> f64 {
    // Sum those together to get a value that determines the region.
    var value: f64;
    
    let in_sum = sum(ins);
    
    if in_sum <= 1.0 {
        // Inside the tetrahedron (3-Simplex) at (0, 0, 0)
        value = inside_tetrahedron_at_0_0_0(ins, in_sum, origin, grid);
    }
    else if in_sum >= 2.0 {
        // Inside the tetrahedron (3-Simplex) at (1, 1, 1)
        value = inside_tetrahedron_at_1_1_1(ins, in_sum, origin, grid);
    }
    else {
        // Inside the octahedron (Rectified 3-Simplex) in between.
        value = inside_octahedron_in_between(ins, origin, grid);
    }

    return value / NORMALIZING_SCALAR;
}

fn inside_tetrahedron_at_1_1_1(
    ins: vec3<f64>,
    in_sum: f64,
    origin: vec3<f64>,
    grid: vec3<f64>,
) -> f64 {
    // Determine which two tetrahedral vertices are the closest, out of (1, 1, 0), (1, 0, 1), (0, 1, 1) but not (1, 1, 1).
    let closest_point = determine_closest_point(
        vec2(ins.x, ins.y),
        vec2(6, 5),
        vec2(3, 3),
        ins,
    );

    // Now we determine the two lattice points not part of the tetrahedron that may contribute.
    // This depends on the closest two tetrahedral vertices, including (1, 1, 1)
    let value = determine_lattice_points_including_1_1_1(
        in_sum,
        closest_point.score,
        closest_point.point,
        origin,
        grid,
    );

    return value
        + contribute(vec3(1.0, 1.0, 0.0), origin, grid)
        + contribute(vec3(1.0, 0.0, 1.0), origin, grid)
        + contribute(vec3(0.0, 1.0, 1.0), origin, grid)
        + contribute(vec3(1.0, 1.0, 1.0), origin, grid);
}

fn determine_lattice_points_including_1_1_1(
    in_sum: f64,
    score: vec2<f64>,
    point: vec2<i32>,
    origin: vec3<f64>,
    grid: vec3<f64>,
) -> f64 {
    let wins = 3.0 - in_sum;
    if wins < score.x || wins < score.y {
        // (1, 1, 1) is one of the closest two tetrahedral vertices.
        // Our other closest vertex is the closest out of a and b.
        var closest: i32;
        if score.y < score.x { closest = point.y; } else { closest = point.x; }
        
        switch closest {
            case 3: {
                return contribute(vec3(2.0, 1.0, 0.0), origin, grid) + contribute(vec3(1.0, 2.0, 0.0), origin, grid);
            }
            case 5: {
                return contribute(vec3(2.0, 0.0, 1.0), origin, grid) + contribute(vec3(1.0, 0.0, 2.0), origin, grid);
            }
            default: {
                return contribute(vec3(0.0, 2.0, 1.0), origin, grid) + contribute(vec3(0.0, 1.0, 2.0), origin, grid); // closest == 6
            }
        }
    } else {
        // (1, 1, 1) is not one of the closest two tetrahedral vertices.
        // Our two extra vertices are determined by the closest two.
        let closest = point.x & point.y;
        switch closest {
            case 1: {
                return contribute(vec3(1.0, 0.0, 0.0), origin, grid) + contribute(vec3(2.0, 0.0, 0.0), origin, grid);
            }
            case 2: {
                return contribute(vec3(0.0, 1.0, 0.0), origin, grid) + contribute(vec3(0.0, 2.0, 0.0), origin, grid);
            }
            default: {
                return contribute(vec3(0.0, 0.0, 1.0), origin, grid) + contribute(vec3(0.0, 0.0, 2.0), origin, grid); // closest == 4
            }
        }
    }
}

struct DetermineFurtherSideResult {
    is_further_side: vec2<bool>, 
    point: vec2<i32>, 
}

fn inside_octahedron_in_between(
    ins: vec3<f64>,
    origin: vec3<f64>,
    grid: vec3<f64>,
) -> f64 {
    let determine_further_side_result = determine_further_side(ins);
    let is_further_side = determine_further_side_result.is_further_side;
    let point = determine_further_side_result.point;

    // Where each of the two closest points are determines how the extra two vertices are calculated.
    var value: f64;
    
    if is_further_side.x == is_further_side.y {
        if is_further_side.x {
            // Both closest points on (1, 1, 1) side
            // One of the two extra points is (1, 1, 1)
            // Other extra point is based on the shared axis.
            let closest = point.x & point.y;

            let cont = contribute(vec3(1.0, 1.0, 1.0), origin, grid);

            switch closest {
                case 1:  { value = cont + contribute(vec3(2.0, 0.0, 0.0), origin, grid); break; }
                case 2:  { value = cont + contribute(vec3(0.0, 2.0, 0.0), origin, grid); break; }
                default: { value = cont + contribute(vec3(0.0, 0.0, 2.0), origin, grid); break; } // closest == 4
            }
        } else {
            // Both closest points on (0, 0, 0) side
            // One of the two extra points is (0, 0, 0)
            // Other extra point is based on the omitted axis.
            let closest = point.x | point.y;

            let cont = contribute(0.0, 0.0, 0.0);

            switch closest {
                case 3:  { value = cont + contribute(vec3(1.0, 1.0, -1.0), origin, grid); break; }
                case 4:  { value = cont + contribute(vec3(1.0, -1.0, 1.0), origin, grid); break; }
                default: { value = cont + contribute(vec3(-1.0, 1.0, 1.0), origin, grid); break; } // closest == 6
            }
        }
    } else {
        // One point on (0, 0, 0) side, one point on (1, 1, 1) side
        var c1: i32;
        var c2: i32;
        if is_further_side.x {
            c1 = point.x;
            c2 = point.y;
        } else {
            c1 = point.y;
            c2 = point.x;
        }

        // One contribution is a permutation of (1, 1, -1)
        // One contribution is a permutation of (0, 0, 2)
        var res: f64;

        switch c1 {
            case 3:  { res = contribute(vec3(1.0, 1.0, -1.0), origin, grid); break; }
            case 5:  { res = contribute(vec3(1.0, -1.0, 1.0), origin, grid); break; }
            default: { res = contribute(vec3(-1.0, 1.0, 1.0), origin, grid); break; } // c1 == 6
        }
        switch c2 {
            case 1:  { value = res + contribute(vec3(2.0, 0.0, 0.0), origin, grid); break; }
            case 2:  { value = res + contribute(vec3(0.0, 2.0, 0.0), origin, grid); break; }
            default: { value = res + contribute(vec3(0.0, 0.0, 2.0), origin, grid); break; } // c1 == 4
        }
    };

    return value
        + contribute(vec3(1.0, 0.0, 0.0), origin, grid)
        + contribute(vec3(0.0, 1.0, 0.0), origin, grid)
        + contribute(vec3(0.0, 0.0, 1.0), origin, grid)
        + contribute(vec3(1.0, 1.0, 0.0), origin, grid)
        + contribute(vec3(1.0, 0.0, 1.0), origin, grid)
        + contribute(vec3(0.0, 1.0, 1.0), origin, grid);
}

struct decide_between_points_inner_ret {
    score: f64,
    point: i32,
    is_further_side: bool
}

fn decide_between_points_inner(p: f64, point_val: vec2<i32>) -> decide_between_points_inner_ret {
    if p > 1.0 {
        return decide_between_points_inner_ret( 
            p - 1.0, 
            point_val.x, 
            true
        );
    }
    
    return decide_between_points_inner_ret( 
        1.0 - p, 
        point_val.y, 
        false
    );
}

struct decide_between_points_ret {
    score: vec2<f64>,
    point: vec2<i32>,
    is_further_side: vec2<bool>
}

fn decide_between_points(ins: vec3<f64>) -> decide_between_points_ret {
    // Decide between point (0, 0, 1) and (1, 1, 0) as closest
    let x = decide_between_points_inner(ins.x + ins.y, vec2(3, 4));
    // Decide between point (0, 1, 0) and (1, 0, 1) as closest
    let y = decide_between_points_inner(ins.x + ins.z, vec2(5, 2));

    return decide_between_points_ret (
        vec2(x.score, y.score),
        vec2(x.point, y.point),
        vec2(x.is_further_side, y.is_further_side),
    );
}

fn determine_further_side(ins: vec3<f64>) -> DetermineFurtherSideResult {
    let decide_result = decide_between_points(ins);
    let score = decide_result.score;
    var point = decide_result.point;
    var is_further_side = decide_result.is_further_side;

    // The closest out of the two (1, 0, 0) and (0, 1, 1) will replace
    // the furthest out of the two decided above, if closer.
    let p = ins.y + ins.z;
    if p > 1.0 {
        let score_value = p - 1.0;
        if score.x <= score.y && score.x < score_value {
            point.x = 6;
            is_further_side.x = true;
        } else if score.x > score.y && score.y < score_value {
            point.y = 6;
            is_further_side.y = true;
        }
    } else {
        let score_value = 1.0 - p;
        if score.x <= score.y && score.x < score_value {
            point.x = 1;
            is_further_side.x = false;
        } else if score.x > score.y && score.y < score_value {
            point.y = 1;
            is_further_side.y = false;
        }
    }

    return DetermineFurtherSideResult(
        is_further_side,
        point
    );
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
fn perm(i: u32) -> u32 {
    return permutation_table[i/4][i%4];
}

// fn perm_grad_index(i: u32) -> u32 {
//     // return permutation_grad_index_table[i/4][i%4];
// }



// # Gradients for 3D. They approximate the directions to the
// # vertices of a rhombicuboctahedron from the center, skewed so
// # that the triangular and square facets can be inscribed inside
// # circles of the same radius.
// const GRADIENTS3: array<i32, 72> = [
//     -11, 4, 4, -4, 11, 4, -4, 4, 11,
//     11, 4, 4, 4, 11, 4, 4, 4, 11,
//     -11, -4, 4, -4, -11, 4, -4, -4, 11,
//     11, -4, 4, 4, -11, 4, 4, -4, 11,
//     -11, 4, -4, -4, 11, -4, -4, 4, -11,
//     11, 4, -4, 4, 11, -4, 4, 4, -11,
//     -11, -4, -4, -4, -11, -4, -4, -4, -11,
//     11, -4, -4, 4, -11, -4, 4, -4, -11,
// ];

/*
// https://github.com/lmas/opensimplex/blob/master/opensimplex/constants.py
const NORM_CONSTANT3: i32 = 103;
const SQUISH_CONSTANT3: f32 = 1.0 / 3.0; // (sqrt(3+1)-1)/3
const STRETCH_CONSTANT3: f32 = -1.0 / 6.0; // (1/sqrt(3+1)-1)/3

fn _extrapolate3(xsb: u32, ysb: u32, zsb: u32, dx: f32, dy: f32, dz: f32) -> f32 {
    let index = perm_grad_index((perm((perm(xsb & 0xFF) + ysb) & 0xFF) + zsb) & 0xFF);
    return GRADIENTS3[index] * dx + GRADIENTS3[index+1] * dy + GRADIENTS3[index+2] * dz;
}

// https://github.com/lmas/opensimplex/blob/master/opensimplex/internals.py#L206
fn _noise3(x, y, z, perm, perm_grad_index3):
    // Place input coordinates on simplectic honeycomb.
    let stretch_offset = (x + y + z) * STRETCH_CONSTANT3
    xs = x + stretch_offset
    ys = y + stretch_offset
    zs = z + stretch_offset

    # Floor to get simplectic honeycomb coordinates of rhombohedron (stretched cube) super-cell origin.
    xsb = floor(xs)
    ysb = floor(ys)
    zsb = floor(zs)

    # Skew out to get actual coordinates of rhombohedron origin. We'll need these later.
    squish_offset = (xsb + ysb + zsb) * SQUISH_CONSTANT3
    xb = xsb + squish_offset
    yb = ysb + squish_offset
    zb = zsb + squish_offset

    # Compute simplectic honeycomb coordinates relative to rhombohedral origin.
    xins = xs - xsb
    yins = ys - ysb
    zins = zs - zsb

    # Sum those together to get a value that determines which region we're in.
    in_sum = xins + yins + zins

    # Positions relative to origin point.
    dx0 = x - xb
    dy0 = y - yb
    dz0 = z - zb

    value = 0
    if in_sum <= 1:  # We're inside the tetrahedron (3-Simplex) at (0,0,0)

        # Determine which two of (0,0,1), (0,1,0), (1,0,0) are closest.
        a_point = 0x01
        a_score = xins
        b_point = 0x02
        b_score = yins
        if a_score >= b_score and zins > b_score:
            b_score = zins
            b_point = 0x04
        elif a_score < b_score and zins > a_score:
            a_score = zins
            a_point = 0x04

        # Now we determine the two lattice points not part of the tetrahedron that may contribute.
        # This depends on the closest two tetrahedral vertices, including (0,0,0)
        wins = 1 - in_sum
        if wins > a_score or wins > b_score:  # (0,0,0) is one of the closest two tetrahedral vertices.
            c = b_point if (b_score > a_score) else a_point  # Our other closest vertex is the closest out of a and b.

            if (c & 0x01) == 0:
                xsv_ext0 = xsb - 1
                xsv_ext1 = xsb
                dx_ext0 = dx0 + 1
                dx_ext1 = dx0
            else:
                xsv_ext0 = xsv_ext1 = xsb + 1
                dx_ext0 = dx_ext1 = dx0 - 1

            if (c & 0x02) == 0:
                ysv_ext0 = ysv_ext1 = ysb
                dy_ext0 = dy_ext1 = dy0
                if (c & 0x01) == 0:
                    ysv_ext1 -= 1
                    dy_ext1 += 1
                else:
                    ysv_ext0 -= 1
                    dy_ext0 += 1
            else:
                ysv_ext0 = ysv_ext1 = ysb + 1
                dy_ext0 = dy_ext1 = dy0 - 1

            if (c & 0x04) == 0:
                zsv_ext0 = zsb
                zsv_ext1 = zsb - 1
                dz_ext0 = dz0
                dz_ext1 = dz0 + 1
            else:
                zsv_ext0 = zsv_ext1 = zsb + 1
                dz_ext0 = dz_ext1 = dz0 - 1
        else:  # (0,0,0) is not one of the closest two tetrahedral vertices.
            c = a_point | b_point  # Our two extra vertices are determined by the closest two.

            if (c & 0x01) == 0:
                xsv_ext0 = xsb
                xsv_ext1 = xsb - 1
                dx_ext0 = dx0 - 2 * SQUISH_CONSTANT3
                dx_ext1 = dx0 + 1 - SQUISH_CONSTANT3
            else:
                xsv_ext0 = xsv_ext1 = xsb + 1
                dx_ext0 = dx0 - 1 - 2 * SQUISH_CONSTANT3
                dx_ext1 = dx0 - 1 - SQUISH_CONSTANT3

            if (c & 0x02) == 0:
                ysv_ext0 = ysb
                ysv_ext1 = ysb - 1
                dy_ext0 = dy0 - 2 * SQUISH_CONSTANT3
                dy_ext1 = dy0 + 1 - SQUISH_CONSTANT3
            else:
                ysv_ext0 = ysv_ext1 = ysb + 1
                dy_ext0 = dy0 - 1 - 2 * SQUISH_CONSTANT3
                dy_ext1 = dy0 - 1 - SQUISH_CONSTANT3

            if (c & 0x04) == 0:
                zsv_ext0 = zsb
                zsv_ext1 = zsb - 1
                dz_ext0 = dz0 - 2 * SQUISH_CONSTANT3
                dz_ext1 = dz0 + 1 - SQUISH_CONSTANT3
            else:
                zsv_ext0 = zsv_ext1 = zsb + 1
                dz_ext0 = dz0 - 1 - 2 * SQUISH_CONSTANT3
                dz_ext1 = dz0 - 1 - SQUISH_CONSTANT3

        # Contribution (0,0,0)
        attn0 = 2 - dx0 * dx0 - dy0 * dy0 - dz0 * dz0
        if attn0 > 0:
            attn0 *= attn0
            value += attn0 * attn0 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 0, zsb + 0, dx0, dy0, dz0)

        # Contribution (1,0,0)
        dx1 = dx0 - 1 - SQUISH_CONSTANT3
        dy1 = dy0 - 0 - SQUISH_CONSTANT3
        dz1 = dz0 - 0 - SQUISH_CONSTANT3
        attn1 = 2 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1
        if attn1 > 0:
            attn1 *= attn1
            value += attn1 * attn1 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 0, zsb + 0, dx1, dy1, dz1)

        # Contribution (0,1,0)
        dx2 = dx0 - 0 - SQUISH_CONSTANT3
        dy2 = dy0 - 1 - SQUISH_CONSTANT3
        dz2 = dz1
        attn2 = 2 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2
        if attn2 > 0:
            attn2 *= attn2
            value += attn2 * attn2 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 1, zsb + 0, dx2, dy2, dz2)

        # Contribution (0,0,1)
        dx3 = dx2
        dy3 = dy1
        dz3 = dz0 - 1 - SQUISH_CONSTANT3
        attn3 = 2 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3
        if attn3 > 0:
            attn3 *= attn3
            value += attn3 * attn3 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 0, zsb + 1, dx3, dy3, dz3)
    elif in_sum >= 2:  # We're inside the tetrahedron (3-Simplex) at (1,1,1)

        # Determine which two tetrahedral vertices are the closest, out of (1,1,0), (1,0,1), (0,1,1) but not (1,1,1).
        a_point = 0x06
        a_score = xins
        b_point = 0x05
        b_score = yins
        if a_score <= b_score and zins < b_score:
            b_score = zins
            b_point = 0x03
        elif a_score > b_score and zins < a_score:
            a_score = zins
            a_point = 0x03

        # Now we determine the two lattice points not part of the tetrahedron that may contribute.
        # This depends on the closest two tetrahedral vertices, including (1,1,1)
        wins = 3 - in_sum
        if wins < a_score or wins < b_score:  # (1,1,1) is one of the closest two tetrahedral vertices.
            c = b_point if (b_score < a_score) else a_point  # Our other closest vertex is the closest out of a and b.

            if (c & 0x01) != 0:
                xsv_ext0 = xsb + 2
                xsv_ext1 = xsb + 1
                dx_ext0 = dx0 - 2 - 3 * SQUISH_CONSTANT3
                dx_ext1 = dx0 - 1 - 3 * SQUISH_CONSTANT3
            else:
                xsv_ext0 = xsv_ext1 = xsb
                dx_ext0 = dx_ext1 = dx0 - 3 * SQUISH_CONSTANT3

            if (c & 0x02) != 0:
                ysv_ext0 = ysv_ext1 = ysb + 1
                dy_ext0 = dy_ext1 = dy0 - 1 - 3 * SQUISH_CONSTANT3
                if (c & 0x01) != 0:
                    ysv_ext1 += 1
                    dy_ext1 -= 1
                else:
                    ysv_ext0 += 1
                    dy_ext0 -= 1
            else:
                ysv_ext0 = ysv_ext1 = ysb
                dy_ext0 = dy_ext1 = dy0 - 3 * SQUISH_CONSTANT3

            if (c & 0x04) != 0:
                zsv_ext0 = zsb + 1
                zsv_ext1 = zsb + 2
                dz_ext0 = dz0 - 1 - 3 * SQUISH_CONSTANT3
                dz_ext1 = dz0 - 2 - 3 * SQUISH_CONSTANT3
            else:
                zsv_ext0 = zsv_ext1 = zsb
                dz_ext0 = dz_ext1 = dz0 - 3 * SQUISH_CONSTANT3
        else:  # (1,1,1) is not one of the closest two tetrahedral vertices.
            c = a_point & b_point  # Our two extra vertices are determined by the closest two.

            if (c & 0x01) != 0:
                xsv_ext0 = xsb + 1
                xsv_ext1 = xsb + 2
                dx_ext0 = dx0 - 1 - SQUISH_CONSTANT3
                dx_ext1 = dx0 - 2 - 2 * SQUISH_CONSTANT3
            else:
                xsv_ext0 = xsv_ext1 = xsb
                dx_ext0 = dx0 - SQUISH_CONSTANT3
                dx_ext1 = dx0 - 2 * SQUISH_CONSTANT3

            if (c & 0x02) != 0:
                ysv_ext0 = ysb + 1
                ysv_ext1 = ysb + 2
                dy_ext0 = dy0 - 1 - SQUISH_CONSTANT3
                dy_ext1 = dy0 - 2 - 2 * SQUISH_CONSTANT3
            else:
                ysv_ext0 = ysv_ext1 = ysb
                dy_ext0 = dy0 - SQUISH_CONSTANT3
                dy_ext1 = dy0 - 2 * SQUISH_CONSTANT3

            if (c & 0x04) != 0:
                zsv_ext0 = zsb + 1
                zsv_ext1 = zsb + 2
                dz_ext0 = dz0 - 1 - SQUISH_CONSTANT3
                dz_ext1 = dz0 - 2 - 2 * SQUISH_CONSTANT3
            else:
                zsv_ext0 = zsv_ext1 = zsb
                dz_ext0 = dz0 - SQUISH_CONSTANT3
                dz_ext1 = dz0 - 2 * SQUISH_CONSTANT3

        # Contribution (1,1,0)
        dx3 = dx0 - 1 - 2 * SQUISH_CONSTANT3
        dy3 = dy0 - 1 - 2 * SQUISH_CONSTANT3
        dz3 = dz0 - 0 - 2 * SQUISH_CONSTANT3
        attn3 = 2 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3
        if attn3 > 0:
            attn3 *= attn3
            value += attn3 * attn3 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 1, zsb + 0, dx3, dy3, dz3)

        # Contribution (1,0,1)
        dx2 = dx3
        dy2 = dy0 - 0 - 2 * SQUISH_CONSTANT3
        dz2 = dz0 - 1 - 2 * SQUISH_CONSTANT3
        attn2 = 2 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2
        if attn2 > 0:
            attn2 *= attn2
            value += attn2 * attn2 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 0, zsb + 1, dx2, dy2, dz2)

        # Contribution (0,1,1)
        dx1 = dx0 - 0 - 2 * SQUISH_CONSTANT3
        dy1 = dy3
        dz1 = dz2
        attn1 = 2 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1
        if attn1 > 0:
            attn1 *= attn1
            value += attn1 * attn1 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 1, zsb + 1, dx1, dy1, dz1)

        # Contribution (1,1,1)
        dx0 = dx0 - 1 - 3 * SQUISH_CONSTANT3
        dy0 = dy0 - 1 - 3 * SQUISH_CONSTANT3
        dz0 = dz0 - 1 - 3 * SQUISH_CONSTANT3
        attn0 = 2 - dx0 * dx0 - dy0 * dy0 - dz0 * dz0
        if attn0 > 0:
            attn0 *= attn0
            value += attn0 * attn0 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 1, zsb + 1, dx0, dy0, dz0)
    else:  # We're inside the octahedron (Rectified 3-Simplex) in between.
        # Decide between point (0,0,1) and (1,1,0) as closest
        p1 = xins + yins
        if p1 > 1:
            a_score = p1 - 1
            a_point = 0x03
            a_is_further_side = True
        else:
            a_score = 1 - p1
            a_point = 0x04
            a_is_further_side = False

        # Decide between point (0,1,0) and (1,0,1) as closest
        p2 = xins + zins
        if p2 > 1:
            b_score = p2 - 1
            b_point = 0x05
            b_is_further_side = True
        else:
            b_score = 1 - p2
            b_point = 0x02
            b_is_further_side = False

        # The closest out of the two (1,0,0) and (0,1,1) will replace the furthest
        # out of the two decided above, if closer.
        p3 = yins + zins
        if p3 > 1:
            score = p3 - 1
            if a_score <= b_score and a_score < score:
                a_point = 0x06
                a_is_further_side = True
            elif a_score > b_score and b_score < score:
                b_point = 0x06
                b_is_further_side = True
        else:
            score = 1 - p3
            if a_score <= b_score and a_score < score:
                a_point = 0x01
                a_is_further_side = False
            elif a_score > b_score and b_score < score:
                b_point = 0x01
                b_is_further_side = False

        # Where each of the two closest points are determines how the extra two vertices are calculated.
        if a_is_further_side == b_is_further_side:
            if a_is_further_side:  # Both closest points on (1,1,1) side

                # One of the two extra points is (1,1,1)
                dx_ext0 = dx0 - 1 - 3 * SQUISH_CONSTANT3
                dy_ext0 = dy0 - 1 - 3 * SQUISH_CONSTANT3
                dz_ext0 = dz0 - 1 - 3 * SQUISH_CONSTANT3
                xsv_ext0 = xsb + 1
                ysv_ext0 = ysb + 1
                zsv_ext0 = zsb + 1

                # Other extra point is based on the shared axis.
                c = a_point & b_point
                if (c & 0x01) != 0:
                    dx_ext1 = dx0 - 2 - 2 * SQUISH_CONSTANT3
                    dy_ext1 = dy0 - 2 * SQUISH_CONSTANT3
                    dz_ext1 = dz0 - 2 * SQUISH_CONSTANT3
                    xsv_ext1 = xsb + 2
                    ysv_ext1 = ysb
                    zsv_ext1 = zsb
                elif (c & 0x02) != 0:
                    dx_ext1 = dx0 - 2 * SQUISH_CONSTANT3
                    dy_ext1 = dy0 - 2 - 2 * SQUISH_CONSTANT3
                    dz_ext1 = dz0 - 2 * SQUISH_CONSTANT3
                    xsv_ext1 = xsb
                    ysv_ext1 = ysb + 2
                    zsv_ext1 = zsb
                else:
                    dx_ext1 = dx0 - 2 * SQUISH_CONSTANT3
                    dy_ext1 = dy0 - 2 * SQUISH_CONSTANT3
                    dz_ext1 = dz0 - 2 - 2 * SQUISH_CONSTANT3
                    xsv_ext1 = xsb
                    ysv_ext1 = ysb
                    zsv_ext1 = zsb + 2
            else:  # Both closest points on (0,0,0) side

                # One of the two extra points is (0,0,0)
                dx_ext0 = dx0
                dy_ext0 = dy0
                dz_ext0 = dz0
                xsv_ext0 = xsb
                ysv_ext0 = ysb
                zsv_ext0 = zsb

                # Other extra point is based on the omitted axis.
                c = a_point | b_point
                if (c & 0x01) == 0:
                    dx_ext1 = dx0 + 1 - SQUISH_CONSTANT3
                    dy_ext1 = dy0 - 1 - SQUISH_CONSTANT3
                    dz_ext1 = dz0 - 1 - SQUISH_CONSTANT3
                    xsv_ext1 = xsb - 1
                    ysv_ext1 = ysb + 1
                    zsv_ext1 = zsb + 1
                elif (c & 0x02) == 0:
                    dx_ext1 = dx0 - 1 - SQUISH_CONSTANT3
                    dy_ext1 = dy0 + 1 - SQUISH_CONSTANT3
                    dz_ext1 = dz0 - 1 - SQUISH_CONSTANT3
                    xsv_ext1 = xsb + 1
                    ysv_ext1 = ysb - 1
                    zsv_ext1 = zsb + 1
                else:
                    dx_ext1 = dx0 - 1 - SQUISH_CONSTANT3
                    dy_ext1 = dy0 - 1 - SQUISH_CONSTANT3
                    dz_ext1 = dz0 + 1 - SQUISH_CONSTANT3
                    xsv_ext1 = xsb + 1
                    ysv_ext1 = ysb + 1
                    zsv_ext1 = zsb - 1
        else:  # One point on (0,0,0) side, one point on (1,1,1) side
            if a_is_further_side:
                c1 = a_point
                c2 = b_point
            else:
                c1 = b_point
                c2 = a_point

            # One contribution is a _permutation of (1,1,-1)
            if (c1 & 0x01) == 0:
                dx_ext0 = dx0 + 1 - SQUISH_CONSTANT3
                dy_ext0 = dy0 - 1 - SQUISH_CONSTANT3
                dz_ext0 = dz0 - 1 - SQUISH_CONSTANT3
                xsv_ext0 = xsb - 1
                ysv_ext0 = ysb + 1
                zsv_ext0 = zsb + 1
            elif (c1 & 0x02) == 0:
                dx_ext0 = dx0 - 1 - SQUISH_CONSTANT3
                dy_ext0 = dy0 + 1 - SQUISH_CONSTANT3
                dz_ext0 = dz0 - 1 - SQUISH_CONSTANT3
                xsv_ext0 = xsb + 1
                ysv_ext0 = ysb - 1
                zsv_ext0 = zsb + 1
            else:
                dx_ext0 = dx0 - 1 - SQUISH_CONSTANT3
                dy_ext0 = dy0 - 1 - SQUISH_CONSTANT3
                dz_ext0 = dz0 + 1 - SQUISH_CONSTANT3
                xsv_ext0 = xsb + 1
                ysv_ext0 = ysb + 1
                zsv_ext0 = zsb - 1

            # One contribution is a _permutation of (0,0,2)
            dx_ext1 = dx0 - 2 * SQUISH_CONSTANT3
            dy_ext1 = dy0 - 2 * SQUISH_CONSTANT3
            dz_ext1 = dz0 - 2 * SQUISH_CONSTANT3
            xsv_ext1 = xsb
            ysv_ext1 = ysb
            zsv_ext1 = zsb
            if (c2 & 0x01) != 0:
                dx_ext1 -= 2
                xsv_ext1 += 2
            elif (c2 & 0x02) != 0:
                dy_ext1 -= 2
                ysv_ext1 += 2
            else:
                dz_ext1 -= 2
                zsv_ext1 += 2

        # Contribution (1,0,0)
        dx1 = dx0 - 1 - SQUISH_CONSTANT3
        dy1 = dy0 - 0 - SQUISH_CONSTANT3
        dz1 = dz0 - 0 - SQUISH_CONSTANT3
        attn1 = 2 - dx1 * dx1 - dy1 * dy1 - dz1 * dz1
        if attn1 > 0:
            attn1 *= attn1
            value += attn1 * attn1 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 0, zsb + 0, dx1, dy1, dz1)

        # Contribution (0,1,0)
        dx2 = dx0 - 0 - SQUISH_CONSTANT3
        dy2 = dy0 - 1 - SQUISH_CONSTANT3
        dz2 = dz1
        attn2 = 2 - dx2 * dx2 - dy2 * dy2 - dz2 * dz2
        if attn2 > 0:
            attn2 *= attn2
            value += attn2 * attn2 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 1, zsb + 0, dx2, dy2, dz2)

        # Contribution (0,0,1)
        dx3 = dx2
        dy3 = dy1
        dz3 = dz0 - 1 - SQUISH_CONSTANT3
        attn3 = 2 - dx3 * dx3 - dy3 * dy3 - dz3 * dz3
        if attn3 > 0:
            attn3 *= attn3
            value += attn3 * attn3 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 0, zsb + 1, dx3, dy3, dz3)

        # Contribution (1,1,0)
        dx4 = dx0 - 1 - 2 * SQUISH_CONSTANT3
        dy4 = dy0 - 1 - 2 * SQUISH_CONSTANT3
        dz4 = dz0 - 0 - 2 * SQUISH_CONSTANT3
        attn4 = 2 - dx4 * dx4 - dy4 * dy4 - dz4 * dz4
        if attn4 > 0:
            attn4 *= attn4
            value += attn4 * attn4 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 1, zsb + 0, dx4, dy4, dz4)

        # Contribution (1,0,1)
        dx5 = dx4
        dy5 = dy0 - 0 - 2 * SQUISH_CONSTANT3
        dz5 = dz0 - 1 - 2 * SQUISH_CONSTANT3
        attn5 = 2 - dx5 * dx5 - dy5 * dy5 - dz5 * dz5
        if attn5 > 0:
            attn5 *= attn5
            value += attn5 * attn5 * _extrapolate3(perm, perm_grad_index3, xsb + 1, ysb + 0, zsb + 1, dx5, dy5, dz5)

        # Contribution (0,1,1)
        dx6 = dx0 - 0 - 2 * SQUISH_CONSTANT3
        dy6 = dy4
        dz6 = dz5
        attn6 = 2 - dx6 * dx6 - dy6 * dy6 - dz6 * dz6
        if attn6 > 0:
            attn6 *= attn6
            value += attn6 * attn6 * _extrapolate3(perm, perm_grad_index3, xsb + 0, ysb + 1, zsb + 1, dx6, dy6, dz6)

    # First extra vertex
    attn_ext0 = 2 - dx_ext0 * dx_ext0 - dy_ext0 * dy_ext0 - dz_ext0 * dz_ext0
    if attn_ext0 > 0:
        attn_ext0 *= attn_ext0
        value += (
            attn_ext0
            * attn_ext0
            * _extrapolate3(perm, perm_grad_index3, xsv_ext0, ysv_ext0, zsv_ext0, dx_ext0, dy_ext0, dz_ext0)
        )

    # Second extra vertex
    attn_ext1 = 2 - dx_ext1 * dx_ext1 - dy_ext1 * dy_ext1 - dz_ext1 * dz_ext1
    if attn_ext1 > 0:
        attn_ext1 *= attn_ext1
        value += (
            attn_ext1
            * attn_ext1
            * _extrapolate3(perm, perm_grad_index3, xsv_ext1, ysv_ext1, zsv_ext1, dx_ext1, dy_ext1, dz_ext1)
        )

    return value / NORM_CONSTANT3
*/




















































































































// Old noise functions


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
// fn hash(i: u32) -> u32 {
//     return permutation_table[i/4][i%4];
// }

// /**
//  * Helper functions to compute gradients-dot-residual vectors (3D)
//  *
//  * @param[in] hash  hash value
//  * @param[in] x     x coord of the distance to the corner
//  * @param[in] y     y coord of the distance to the corner
//  * @param[in] z     z coord of the distance to the corner
//  *
//  * @return gradient value
//  */
// fn grad(hash: u32, x: f64, y: f64, z: f64) -> f64 {
//     let h = hash & 15;
//     let hl8 = f64(h < 8); // Convert low 4 bits of hash code into 12 simple
//     let u = (hl8 * x) + ((1.0 - hl8) * y); // gradient directions, and compute dot product.
//     let hl4 = f64(h < 4);
//     let otr = f64(h == 12 || h == 14);
//     let v = (hl4 * y) + (1.0 - hl4) * (otr * x) + (1.0 - otr) * z; // Fix repeats at h = 12 to 15

//     let hand1 = f64(h & 1);
//     let hand2 = f64(h & 2);

//     return (hand1 * -1.0 + (1.0 - hand1)) * u + (hand2 * -1.0 + (1.0 - hand2)) * v;
// }


// // Skewing/Unskewing factors for 3D
// const F3 = f64(1.0 / 3.0);
// const G3 = f64(1.0 / 6.0);

// /**
//  * Translated from: https://github.com/SRombauts/SimplexNoise/blob/master/src/SimplexNoise.cpp
//  * 
//  * 3D Perlin simplex noise
//  *
//  * @param[in] x float coordinate
//  * @param[in] y float coordinate
//  * @param[in] z float coordinate
//  *
//  * @return Noise value in the range[-1; 1], value of 0 on all integer coordinates.
//  */
// fn noise(x: f64, y: f64, z: f64) -> f64 {
//     var n0 = f64(0.0);
//     var n1 = f64(0.0);
//     var n2 = f64(0.0);
//     var n3 = f64(0.0); // Noise contributions from the four corners

//     // Skew the input space to determine which simplex cell we're in
//     let s = (x + y + z) * F3; // Very nice and simple skew factor for 3D
//     let i = u32(floor(x + s));
//     let j = u32(floor(y + s));
//     let k = u32(floor(z + s));
//     let t = f64((i + j + k)) * G3;
//     let X0 = f64(i) - t; // Unskew the cell origin back to (x,y,z) space
//     let Y0 = f64(j) - t;
//     let Z0 = f64(k) - t;
//     let x0 = x - X0; // The x,y,z distances from the cell origin
//     let y0 = y - Y0;
//     let z0 = z - Z0;

//     // For the 3D case, the simplex shape is a slightly irregular tetrahedron.
//     // Determine which simplex we are in.
//     var i1 = 0;
//     var j1 = 0;
//     var k1 = 0; // Offsets for second corner of simplex in (i,j,k) coords
//     var i2 = 0;
//     var j2 = 0;
//     var k2 = 0; // Offsets for third corner of simplex in (i,j,k) coords
//     if x0 >= y0 {
//         if y0 >= z0 {
//             i1 = 1;
//             j1 = 0;
//             k1 = 0;
//             i2 = 1;
//             j2 = 1;
//             k2 = 0; // X Y Z order
//         } else if x0 >= z0 {
//             i1 = 1;
//             j1 = 0;
//             k1 = 0;
//             i2 = 1;
//             j2 = 0;
//             k2 = 1; // X Z Y order
//         } else {
//             i1 = 0;
//             j1 = 0;
//             k1 = 1;
//             i2 = 1;
//             j2 = 0;
//             k2 = 1; // Z X Y order
//         }
//     } else {
//         // x0<y0
//         if y0 < z0 {
//             i1 = 0;
//             j1 = 0;
//             k1 = 1;
//             i2 = 0;
//             j2 = 1;
//             k2 = 1; // Z Y X order
//         } else if x0 < z0 {
//             i1 = 0;
//             j1 = 1;
//             k1 = 0;
//             i2 = 0;
//             j2 = 1;
//             k2 = 1; // Y Z X order
//         } else {
//             i1 = 0;
//             j1 = 1;
//             k1 = 0;
//             i2 = 1;
//             j2 = 1;
//             k2 = 0; // Y X Z order
//         }
//     }

//     // A step of (1,0,0) in (i,j,k) means a step of (1-c,-c,-c) in (x,y,z),
//     // a step of (0,1,0) in (i,j,k) means a step of (-c,1-c,-c) in (x,y,z), and
//     // a step of (0,0,1) in (i,j,k) means a step of (-c,-c,1-c) in (x,y,z), where
//     // c = 1/6.
//     let x1 = x0 - f64(i1) + G3; // Offsets for second corner in (x,y,z) coords
//     let y1 = y0 - f64(j1) + G3;
//     let z1 = z0 - f64(k1) + G3;
//     let x2 = x0 - f64(i2) + 2.0 * G3; // Offsets for third corner in (x,y,z) coords
//     let y2 = y0 - f64(j2) + 2.0 * G3;
//     let z2 = z0 - f64(k2) + 2.0 * G3;
//     let x3 = x0 - 1.0 + 3.0 * G3; // Offsets for last corner in (x,y,z) coords
//     let y3 = y0 - 1.0 + 3.0 * G3;
//     let z3 = z0 - 1.0 + 3.0 * G3;

//     // Work out the hashed gradient indices of the four simplex corners
//     let gi0 = hash(u32(i) + hash(u32(j) + hash(k)));
//     let gi1 = hash(u32(i) + u32(i1) + hash(u32(j) + u32(j1) + hash(u32(k) + u32(k1))));
//     let gi2 = hash(u32(i) + u32(i2) + hash(u32(j) + u32(j2) + hash(u32(k) + u32(k2))));
//     let gi3 = hash(u32(i) + u32(1) + hash(u32(j) + u32(1) + hash(u32(k) + u32(1))));

//     // Calculate the contribution from the four corners
//     var t0 = f64(0.6 - x0 * x0 - y0 * y0 - z0 * z0);
//     if t0 < 0.0 {
//         n0 = f64(0.0);
//     } else {
//         t0 *= t0;
//         n0 = t0 * t0 * grad(gi0, x0, y0, z0);
//     }
//     var t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
//     if t1 < 0.0 {
//         n1 = f64(0.0);
//     } else {
//         t1 *= t1;
//         n1 = t1 * t1 * grad(gi1, x1, y1, z1);
//     }
//     var t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
//     if t2 < 0.0 {
//         n2 = f64(0.0);
//     } else {
//         t2 *= t2;
//         n2 = t2 * t2 * grad(gi2, x2, y2, z2);
//     }
//     var t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
//     if t3 < 0.0 {
//         n3 = f64(0.0);
//     } else {
//         t3 *= t3;
//         n3 = t3 * t3 * grad(gi3, x3, y3, z3);
//     }
//     // Add contributions from each corner to get the final noise value.
//     // The result is scaled to stay just inside [-1,1]
//     return 32.0 * (n0 + n1 + n2 + n3);
// }