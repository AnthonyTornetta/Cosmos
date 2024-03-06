//! Handles the initialization of the server world

use std::{
    fs,
    num::Wrapping,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use bevy::prelude::*;
use cosmos_core::netty::cosmos_encoder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Resource, Deref, Serialize, Deserialize, Clone, Copy)]
/// This sets the seed the server uses to generate the universe
pub struct ServerSeed(u64);

impl ServerSeed {
    /// Gets the u64 representation of this seed
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Gets the u32 representation of this seed
    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    /// Computes a "random" number at the given x, y, z coordinates.
    ///
    /// This randomness is based off a hash of the coordinates with this seed.
    pub fn chaos_hash(&self, x: f64, y: f64, z: f64) -> i64 {
        let wrapping_seed = Wrapping(self.0 as i64);

        let mut h =
            wrapping_seed + (Wrapping((x * 374761393.0) as i64) + Wrapping((y * 668265263.0) as i64) + Wrapping((z * 1610612741.0) as i64)); //all constants are prime

        h = (h ^ (h >> 13)) * Wrapping(1274126177);
        (h ^ Wrapping(h.0 >> 16)).0
    }
}

#[derive(Resource, Debug, Clone, Deref, DerefMut, Default)]
/// A pre-seeded structure to create noise values. Uses simplex noise as the backend
///
/// This cannot be sent across threads - use ReadOnlyNoise to send use across threads.
pub struct Noise(noise::OpenSimplex);

impl Noise {
    /// Creates a new noise based on the seed you provide
    pub fn new(seed: u32) -> Self {
        Self(noise::OpenSimplex::new(seed))
    }
}

#[derive(Resource, Debug, Clone)]
/// A thread-safe pre-seeded structure to create noise values. Uses simplex noise as the backend
///
/// To use across threads, just clone this and call the `inner` method to get the Noise struct this encapsulates
pub struct ReadOnlyNoise(Arc<RwLock<Noise>>);

impl ReadOnlyNoise {
    /// Returns the `Noise` instance this encapsulates
    pub fn inner(&self) -> RwLockReadGuard<Noise> {
        self.0.read().expect("Failed to read")
    }
}

pub(super) fn register(app: &mut App) {
    let server_seed = if let Ok(seed) = fs::read("./world/seed.dat") {
        cosmos_encoder::deserialize::<ServerSeed>(&seed).expect("Unable to understand './world/seed.dat' seed file. Is it corrupted?")
    } else {
        let seed = ServerSeed(rand::random());

        fs::create_dir("./world/").expect("Error creating world directory!");
        fs::write("./world/seed.dat", cosmos_encoder::serialize(&seed)).expect("Error writing file './world/seed.dat'");

        seed
    };

    let noise = Noise(noise::OpenSimplex::new(server_seed.as_u32()));
    let read_noise = ReadOnlyNoise(Arc::new(RwLock::new(noise.clone())));

    app.insert_resource(noise).insert_resource(read_noise).insert_resource(server_seed);
}

const perm: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69, 142, 8, 99, 37, 240, 21, 10, 23, 190, 6,
    148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88, 237, 149, 56, 87, 174, 20, 125, 136, 171,
    168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83, 111, 229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55, 46,
    245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80, 73, 209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159,
    86, 164, 100, 109, 198, 173, 186, 3, 64, 52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59, 227,
    47, 16, 58, 17, 182, 189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22,
    39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210, 144, 12, 191, 179,
    162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204, 176, 115, 121, 50, 45, 127, 4, 150,
    254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66, 215, 61, 156, 180,
];

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
fn hash(i: usize) -> u8 {
    perm[(i as u8) as usize]
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
fn grad(hash: i32, x: f64, y: f64, z: f64) -> f64 {
    let h = hash & 15;
    let hl8 = if h < 8 { 1.0 } else { 0.0 }; // Convert low 4 bits of hash code into 12 simple
    let u = (hl8 * x) + ((1.0 - hl8) * y); // gradient directions, and compute dot product.
    let hl4 = if h < 4 { 1.0 } else { 0.0 };
    let otr = if h == 12 || h == 14 { 1.0 } else { 0.0 };
    let v = (hl4 * y) + (1.0 - hl4) * (otr * x) + (1.0 - otr) * z; // Fix repeats at h = 12 to 15

    let hand1 = if (h & 1) == 1 { -1.0 } else { 1.0 };
    let hand2 = if (h & 2) == 1 { -1.0 } else { 1.0 };

    return (hand1 * u) + (hand2 * v);
}

/**
 * 3D Perlin simplex noise
 *
 * @param[in] x float coordinate
 * @param[in] y float coordinate
 * @param[in] z float coordinate
 *
 * @return Noise value in the range[-1; 1], value of 0 on all integer coordinates.
 */
fn noise(x: f64, y: f64, z: f64) -> f64 {
    let n0;
    let n1;
    let n2;
    let n3; // Noise contributions from the four corners

    // Skewing/Unskewing factors for 3D
    const F3: f64 = 1.0 / 3.0;
    const G3: f64 = 1.0 / 6.0;

    // Skew the input space to determine which simplex cell we're in
    let s = (x + y + z) * F3; // Very nice and simple skew factor for 3D
    let i = (x + s).floor() as usize;
    let j = (y + s).floor() as usize;
    let k = (z + s).floor() as usize;
    let t = (i + j + k) as f64 * G3;
    let X0 = i as f64 - t; // Unskew the cell origin back to (x,y,z) space
    let Y0 = j as f64 - t;
    let Z0 = k as f64 - t;
    let x0 = x - X0; // The x,y,z distances from the cell origin
    let y0 = y - Y0;
    let z0 = z - Z0;

    // For the 3D case, the simplex shape is a slightly irregular tetrahedron.
    // Determine which simplex we are in.
    let (i1, j1, k1); // Offsets for second corner of simplex in (i,j,k) coords
    let (i2, j2, k2); // Offsets for third corner of simplex in (i,j,k) coords
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
    let x1 = x0 - i1 as f64 + G3; // Offsets for second corner in (x,y,z) coords
    let y1 = y0 - j1 as f64 + G3;
    let z1 = z0 - k1 as f64 + G3;
    let x2 = x0 - i2 as f64 + 2.0 * G3; // Offsets for third corner in (x,y,z) coords
    let y2 = y0 - j2 as f64 + 2.0 * G3;
    let z2 = z0 - k2 as f64 + 2.0 * G3;
    let x3 = x0 - 1.0 + 3.0 * G3; // Offsets for last corner in (x,y,z) coords
    let y3 = y0 - 1.0 + 3.0 * G3;
    let z3 = z0 - 1.0 + 3.0 * G3;

    // Work out the hashed gradient indices of the four simplex corners
    let gi0 = hash(i + hash(j + hash(k) as usize) as usize);
    let gi1 = hash(i + i1 + hash(j + j1 + hash(k + k1) as usize) as usize);
    let gi2 = hash(i + i2 + hash(j + j2 + hash(k + k2) as usize) as usize);
    let gi3 = hash(i + 1 + hash(j + 1 + hash(k + 1) as usize) as usize);

    // Calculate the contribution from the four corners
    let mut t0 = 0.6 - x0 * x0 - y0 * y0 - z0 * z0;
    if t0 < 0.0 {
        n0 = 0.0;
    } else {
        t0 *= t0;
        n0 = t0 * t0 * grad(gi0 as i32, x0, y0, z0);
    }
    let mut t1 = 0.6 - x1 * x1 - y1 * y1 - z1 * z1;
    if t1 < 0.0 {
        n1 = 0.0;
    } else {
        t1 *= t1;
        n1 = t1 * t1 * grad(gi1 as i32, x1, y1, z1);
    }
    let mut t2 = 0.6 - x2 * x2 - y2 * y2 - z2 * z2;
    if t2 < 0.0 {
        n2 = 0.0;
    } else {
        t2 *= t2;
        n2 = t2 * t2 * grad(gi2 as i32, x2, y2, z2);
    }
    let mut t3 = 0.6 - x3 * x3 - y3 * y3 - z3 * z3;
    if t3 < 0.0 {
        n3 = 0.0;
    } else {
        t3 *= t3;
        n3 = t3 * t3 * grad(gi3 as i32, x3, y3, z3);
    }
    // Add contributions from each corner to get the final noise value.
    // The result is scaled to stay just inside [-1,1]
    return 32.0 * (n0 + n1 + n2 + n3);
}
