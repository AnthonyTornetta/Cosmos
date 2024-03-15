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
fn hash(i: u32) -> u32 {
    return permutation_table[i/4][i%4];
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
    let gi0 = hash(u32(i) + hash(u32(j) + hash(k)));
    let gi1 = hash(u32(i) + u32(i1) + hash(u32(j) + u32(j1) + hash(u32(k) + u32(k1))));
    let gi2 = hash(u32(i) + u32(i2) + hash(u32(j) + u32(j2) + hash(u32(k) + u32(k2))));
    let gi3 = hash(u32(i) + u32(1) + hash(u32(j) + u32(1) + hash(u32(k) + u32(1))));

    // Calculate the contribution from the four corners
    var t0 = f64(0.6 - x0 * x0 - y0 * y0 - z0 * z0);
    if t0 < 0.0 {
        n0 = f64(0.0);
    } else {
        t0 *= t0;
        n0 = t0 * t0 * grad(gi0, x0, y0, z0);
    }
    var t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 < 0.0 {
        n1 = f64(0.0);
    } else {
        t1 *= t1;
        n1 = t1 * t1 * grad(gi1, x1, y1, z1);
    }
    var t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 < 0.0 {
        n2 = f64(0.0);
    } else {
        t2 *= t2;
        n2 = t2 * t2 * grad(gi2, x2, y2, z2);
    }
    var t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 < 0.0 {
        n3 = f64(0.0);
    } else {
        t3 *= t3;
        n3 = t3 * t3 * grad(gi3, x3, y3, z3);
    }
    // Add contributions from each corner to get the final noise value.
    // The result is scaled to stay just inside [-1,1]
    return 32.0 * (n0 + n1 + n2 + n3);
}