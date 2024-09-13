const GRAD_TABLE_LEN: u32 = 24;

@group(0) @binding(0) var<uniform> permutation_table: array<vec4<u32>, 512>; // 2048/4

// This should be constant defined in this shader, but wgpu hates cool things like indexing
// a constant array dynamically.
@group(0) @binding(4) var<uniform> grad_table: array<vec3f, GRAD_TABLE_LEN>;

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
    let delta = f64(0.001);

    let amplitude_delta = f64(0.01);
    let amplitude = abs(noise(
        f64(coords_f32.x + 537.0) * amplitude_delta,
        f64(coords_f32.y - 1123.0) * amplitude_delta,
        f64(coords_f32.z + 1458.0) * amplitude_delta,
    )) * 3.0;

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

    var new_depth: f32 = f32(depth);
    let sign_depth = sign(new_depth);
    new_depth = pow(abs(new_depth), 1.5) * sign_depth;
    // new_depth = pow(new_depth, 1.1);

    var coord: f32 = coords_f32.x;

    let face = planet_face_relative(vec3(coords_f32.x, coords_f32.y, coords_f32.z));

    if face == BF_TOP || face == BF_BOTTOM {
        coord = coords_f32.y;
    } else if face == BF_FRONT || face == BF_BACK {
        coord = coords_f32.z;
    }

    let depth_here = f32(sea_level) + f32(new_depth);

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


fn extrapolate(grid: vec3<f64>, delta: vec3<f64>) -> f64 {
    let point = grad_table[get_grad_table_index(grid)];

    return f64(point.x) * delta.x + f64(point.y) * delta.y + f64(point.z) * delta.z;
}

fn noise(x: f64, y: f64, z: f64) -> f64 {
    let input: vec3<f64> = vec3(x, y, z);
    // -STRETCH_POINT causes a compiler error. idk why
    let stretch: vec3<f64> = input + ((0.0 - STRETCH_POINT) * (input.x + input.y + input.z));
    let grid = floor(stretch);

    let squashed: vec3<f64> = grid + (SQUISH_POINT * (grid.x + grid.y + grid.z));
    let ins = stretch - grid;
    let origin = input - squashed;

    return get_value(grid, origin, ins);
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
        return (attn * attn * attn * attn) * extrapolate(grid + delta, shifted);
    }

    return f64(0.0);
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

    return ClosestPoint(score_mut, point_mut);
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

    return value + contribute(vec3<f64>(0.0, 0.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 0.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 0.0, 1.0), origin, grid);
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
            closest = point.y;
        } else {
            closest = point.x;
        };

        switch closest {
            case 1: {
                return contribute(vec3<f64>(1.0, -1.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 0.0, -1.0), origin, grid);
            }
            case 2: {
                return contribute(vec3<f64>(-1.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, -1.0), origin, grid);
            }
            default: {
                return contribute(vec3<f64>(-1.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(0.0, -1.0, 1.0), origin, grid); // closest == 4
            }
        }
    } else {
        // (0, 0, 0) is not one of the closest two tetrahedral vertices.
        // Our two extra vertices are determined by the closest two.
        let closest = point.x | point.y;
        switch closest {
            case 3: {
                return contribute(vec3<f64>(1.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 1.0, -1.0), origin, grid);
            }
            case 5: {
                return contribute(vec3<f64>(1.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(1.0, -1.0, 1.0), origin, grid);
            }
            default: {
                return contribute(vec3<f64>(0.0, 1.0, 1.0), origin, grid) + contribute(vec3<f64>(-1.0, 1.0, 1.0), origin, grid); // closest == 6
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
    } else if in_sum >= 2.0 {
        // Inside the tetrahedron (3-Simplex) at (1, 1, 1)
        value = inside_tetrahedron_at_1_1_1(ins, in_sum, origin, grid);
    } else {
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

    return value + contribute(vec3<f64>(1.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, 1.0), origin, grid) + contribute(vec3<f64>(1.0, 1.0, 1.0), origin, grid);
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
                return contribute(vec3<f64>(2.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 2.0, 0.0), origin, grid);
            }
            case 5: {
                return contribute(vec3<f64>(2.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(1.0, 0.0, 2.0), origin, grid);
            }
            default: {
                return contribute(vec3<f64>(0.0, 2.0, 1.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, 2.0), origin, grid); // closest == 6
            }
        }
    } else {
        // (1, 1, 1) is not one of the closest two tetrahedral vertices.
        // Our two extra vertices are determined by the closest two.
        let closest = point.x & point.y;
        switch closest {
            case 1: {
                return contribute(vec3<f64>(1.0, 0.0, 0.0), origin, grid) + contribute(vec3<f64>(2.0, 0.0, 0.0), origin, grid);
            }
            case 2: {
                return contribute(vec3<f64>(0.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 2.0, 0.0), origin, grid);
            }
            default: {
                return contribute(vec3<f64>(0.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(0.0, 0.0, 2.0), origin, grid); // closest == 4
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

            let cont = contribute(vec3<f64>(1.0, 1.0, 1.0), origin, grid);

            switch closest {
                case 1:  { value = cont + contribute(vec3<f64>(2.0, 0.0, 0.0), origin, grid); break; }
                case 2:  { value = cont + contribute(vec3<f64>(0.0, 2.0, 0.0), origin, grid); break; }
                default: { value = cont + contribute(vec3<f64>(0.0, 0.0, 2.0), origin, grid); break; } // closest == 4
            }
        } else {
            // Both closest points on (0, 0, 0) side
            // One of the two extra points is (0, 0, 0)
            // Other extra point is based on the omitted axis.
            let closest = point.x | point.y;

            let cont = contribute(vec3<f64>(0.0, 0.0, 0.0), origin, grid);

            switch closest {
                case 3:  { value = cont + contribute(vec3<f64>(1.0, 1.0, -1.0), origin, grid); break; }
                case 4:  { value = cont + contribute(vec3<f64>(1.0, -1.0, 1.0), origin, grid); break; }
                default: { value = cont + contribute(vec3<f64>(-1.0, 1.0, 1.0), origin, grid); break; } // closest == 6
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
            case 3:  { res = contribute(vec3<f64>(1.0, 1.0, -1.0), origin, grid); break; }
            case 5:  { res = contribute(vec3<f64>(1.0, -1.0, 1.0), origin, grid); break; }
            default: { res = contribute(vec3<f64>(-1.0, 1.0, 1.0), origin, grid); break; } // c1 == 6
        }
        switch c2 {
            case 1:  { value = res + contribute(vec3<f64>(2.0, 0.0, 0.0), origin, grid); break; }
            case 2:  { value = res + contribute(vec3<f64>(0.0, 2.0, 0.0), origin, grid); break; }
            default: { value = res + contribute(vec3<f64>(0.0, 0.0, 2.0), origin, grid); break; } // c1 == 4
        }
    };

    return value + contribute(vec3<f64>(1.0, 0.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(0.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(1.0, 1.0, 0.0), origin, grid) + contribute(vec3<f64>(1.0, 0.0, 1.0), origin, grid) + contribute(vec3<f64>(0.0, 1.0, 1.0), origin, grid);
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

    return decide_between_points_ret(
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

fn perm(i: u32) -> u32 {
    return permutation_table[i / 4][i % 4];
}
