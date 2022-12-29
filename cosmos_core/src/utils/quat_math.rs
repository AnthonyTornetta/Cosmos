use std::f64::consts::PI;

use bevy::prelude::Vec3;
use bevy_rapier3d::na::{Quaternion, UnitVector3, Vector3};

// https://stackoverflow.com/questions/1171849/finding-quaternion-representing-the-rotation-from-one-vector-to-another
pub fn quaternion_between_normalized_vectors(
    normalized_from: &Vec3,
    normalized_to: &Vec3,
) -> Quaternion<f64> {
    let normalized_from_f64 = Vector3::new(
        normalized_from.x as f64,
        normalized_from.y as f64,
        normalized_from.z as f64,
    );
    let normalized_to_f64 = &Vector3::new(
        normalized_to.x as f64,
        normalized_to.y as f64,
        normalized_to.z as f64,
    );

    let dot = normalized_from_f64.dot(normalized_to_f64);

    if dot < -0.999999 {
        Quaternion::from_polar_decomposition(
            1.0,
            PI,
            UnitVector3::new_unchecked(Vector3::new(1.0, 0.0, 0.0)),
        )
    } else if dot > 0.999999 {
        Quaternion::identity()
    } else {
        let cross = normalized_from_f64.cross(normalized_to_f64);

        Quaternion::from_parts(1.0 + dot, Vector3::new(cross.x, cross.y, cross.z))
    }
}
