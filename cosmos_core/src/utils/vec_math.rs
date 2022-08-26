use rapier3d::na::Vector3;

#[inline]
pub fn add_vec<T : Copy + std::ops::Add<Output = T>>(v1: &Vector3<T>, v2: &Vector3<T>) -> Vector3<T> {
    Vector3::new(v1[0] + v2[0], v1[1] + v2[1], v1[2] + v2[2])
}