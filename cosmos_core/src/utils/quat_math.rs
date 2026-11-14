//! Different math stuff for quaternions

use bevy::{
    math::{Mat3, Quat},
    prelude::Vec3,
};
use rand::{Rng, RngExt};

/// https://stackoverflow.com/questions/1171849/finding-quaternion-representing-the-rotation-from-one-vector-to-another
///
/// Calculates the quaternion between normalized vectors.
pub fn quaternion_between_normalized_vectors(normalized_from: Vec3, normalized_to: Vec3) -> Quat {
    let dot = normalized_from.dot(normalized_to);

    if dot < -0.999999 {
        Quat::from_axis_angle(Vec3::new(1.0, 0.0, 0.0), 0.0)
    } else if dot > 0.999999 {
        Quat::IDENTITY
    } else {
        let cross = normalized_from.cross(normalized_to);

        Quat::from_xyzw(cross.x, cross.y, cross.z, 1.0 + dot)
    }
}

/// Additional math functions for [`Quat`]s.
pub trait QuatMath {
    /// Creates a quaternion that this multiplied by [`Vec3::Z`] points in the given `direction`
    /// and [`Vec3::Y`] points towards `up`.
    ///
    /// In some cases it's not possible to construct a rotation. Another axis will be picked in those cases:
    /// * if `direction` is zero, `Vec3::NEG_Z` is used instead
    /// * if `up` is zero, `Vec3::Y` is used instead
    /// * if `direction` is parallel with `up`, an orthogonal vector is used as the "right" direction
    fn looking_to(direction: Vec3, up: Vec3) -> Self;
}

impl QuatMath for Quat {
    // stolen from: https://docs.rs/bevy_transform/0.13.2/src/bevy_transform/components/transform.rs.html#359
    fn looking_to(direction: Vec3, up: Vec3) -> Self {
        let back = -direction.try_normalize().unwrap_or(Vec3::NEG_Z);
        let up = up.try_normalize().unwrap_or(Vec3::Y);
        let right = up.cross(back).try_normalize().unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        Quat::from_mat3(&Mat3::from_cols(right, up, back))
    }
}
//
// /// https://stackoverflow.com/questions/31600717/how-to-generate-a-random-quaternion-quickly
// pub fn random_quat(rng: &mut ChaCha8Rng) -> Quat {
//     use std::f32::consts::PI;
//
//     let u = rng.random::<f32>();
//     let v = rng.random::<f32>();
//     let w = rng.random::<f32>();
//
//     Quat::from_xyzw(
//         (1.0 - u).sqrt() * (2.0 * PI * v).sin(),
//         (1.0 - u).sqrt() * (2.0 * PI * v).cos(),
//         u.sqrt() * (2.0 * PI * w).sin(),
//         u.sqrt() * (2.0 * PI * w).cos(),
//     )
// }

/// https://stackoverflow.com/questions/31600717/how-to-generate-a-random-quaternion-quickly
pub fn random_quat<T: Rng>(rng: &mut T) -> Quat {
    use std::f32::consts::PI;

    let u = rng.random::<f32>();
    let v = rng.random::<f32>();
    let w = rng.random::<f32>();

    Quat::from_xyzw(
        (1.0 - u).sqrt() * (2.0 * PI * v).sin(),
        (1.0 - u).sqrt() * (2.0 * PI * v).cos(),
        u.sqrt() * (2.0 * PI * w).sin(),
        u.sqrt() * (2.0 * PI * w).cos(),
    )
}
