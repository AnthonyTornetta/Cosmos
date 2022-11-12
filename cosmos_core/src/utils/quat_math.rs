use std::f32::consts::PI;

use bevy::prelude::{Quat, Vec3};

use super::vec_math::{cross, dot};

/// Calculates the quaternion to rotate from the first argument to the second argument.
///
/// Derived from: https://www.xarg.org/proof/quaternion-from-two-vectors/
pub fn quaternion_between_vectors(from: &Vec3, to: &Vec3) -> Quat {
    let d = dot(from, to);
    let w = cross(from, to);

    Quat::from_axis_angle(w, d + (d * d + dot(&w, &w)).sqrt()).normalize()
}

// https://stackoverflow.com/questions/1171849/finding-quaternion-representing-the-rotation-from-one-vector-to-another
pub fn quaternion_between_normalized_vectors(normalized_from: &Vec3, normalized_to: &Vec3) -> Quat {
    let dot = dot(normalized_from, normalized_to);

    if dot < -0.999 {
        return Quat::from_euler(bevy::prelude::EulerRot::ZYX, 0.0, 0.0, PI);
    } else if dot > 0.999 {
        return Quat::IDENTITY;
    } else {
        let cross = cross(normalized_from, normalized_to);

        return Quat::from_xyzw(cross.x, cross.y, cross.z, 1.0 + dot);
    }
}
