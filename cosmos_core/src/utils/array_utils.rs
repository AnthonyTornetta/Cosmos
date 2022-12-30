#[inline]
pub fn flatten(x: usize, y: usize, z: usize, width: usize, height: usize) -> usize {
    z * width * height + y * width + x
}
