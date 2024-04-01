//! Some array utility functions

#[inline(always)]
/// Calcuates the analogous index for a 1d array given the x/y/z for a 3d array.
pub fn flatten_4d(x: usize, y: usize, z: usize, w: usize, width: usize, height: usize, length: usize) -> usize {
    ((w * length + z) * height + y) * width + x
}

#[inline(always)]
/// Calcuates the analogous index for a 1d array given the x/y/z for a 3d array.
pub fn flatten(x: usize, y: usize, z: usize, width: usize, height: usize) -> usize {
    (z * height + y) * width + x
}

#[inline(always)]
/// Calcuates the analogous index for a 1d array given the x/y/z for a 3d array.
pub fn flatten_2d(x: usize, y: usize, width: usize) -> usize {
    y * width + x
}

/// Reverses the operation of flatten, and gives the 3d x/y/z coordinates for a 3d array given a 1d array coordinate
pub fn expand(index: usize, width: usize, height: usize) -> (usize, usize, usize) {
    let wh = width * height;

    let z = index / wh;
    let y = (index - z * wh) / (width);
    let x = (index - z * wh) - y * width;

    (x, y, z)
}

/// Reverses the operation of flatten, and gives the 4d x/y/z coordinates for a 4d array given a 1d array coordinate
pub fn expand_4(index: usize, width: usize, height: usize, length: usize) -> (usize, usize, usize, usize) {
    let whl = width * height * length;
    let wh = width * height;

    let w = index / whl;
    let z = (index - w * whl) / wh;
    let y = ((index - w * whl) - z * wh) / width;
    let x = ((index - w * whl) - z * wh) - y * width;

    (x, y, z, w)
}

#[cfg(test)]
mod test {
    use crate::utils::array_utils::{expand, flatten};

    #[test]
    fn test() {
        const NUM: usize = 5342512;

        let (x, y, z) = expand(NUM, 23, 50);

        assert_eq!(flatten(x, y, z, 23, 50), NUM);
    }
}
