use bevy::prelude::Vec3;
use bevy_rapier3d::na::Vector3;

#[inline]
pub fn add_vec<T: Copy + std::ops::Add<Output = T>>(
    v1: &Vector3<T>,
    v2: &Vector3<T>,
) -> Vector3<T> {
    Vector3::new(v1[0] + v2[0], v1[1] + v2[1], v1[2] + v2[2])
}

#[inline]
/// Computes dot product of two vectors
///
/// For some reason Vec3::dot(v) takes ownership of v. Use this to avoid that behavior
pub fn dot(a: &Vec3, b: &Vec3) -> f32 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

#[inline]
/// Computes cross product of two vectors
///
/// For some reason Vec3::cross(v) takes ownership of v. Use this to avoid that behavior
pub fn cross(lhs: &Vec3, rhs: &Vec3) -> Vec3 {
    Vec3 {
        x: lhs.y * rhs.z - rhs.y * lhs.z,
        y: lhs.z * rhs.x - rhs.z * lhs.x,
        z: lhs.x * rhs.y - rhs.x * lhs.y,
    }
}
