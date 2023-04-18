//! Some array utility functions

#[inline]
/// Calcuates the analogous index for a 1d array given the x/y/z for a 3d array.
pub fn flatten(x: usize, y: usize, z: usize, width: usize, height: usize) -> usize {
    z * width * height + y * width + x
}
