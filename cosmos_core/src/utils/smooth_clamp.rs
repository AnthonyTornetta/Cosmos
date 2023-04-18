//! Lerp + clamp at the same time

use bevy::prelude::Vec3;

/// Lerp + clamp at the same time
pub trait SmoothClamp {
    /// Clamps this between two other values, but instead of immediately jumping to it,
    ///
    /// lerp is used to slowly move to be within the range.
    fn smooth_clamp(&self, min: &Self, max: &Self, lerp: f32) -> Self;
}

impl SmoothClamp for Vec3 {
    fn smooth_clamp(&self, min: &Self, max: &Self, lerp: f32) -> Self {
        debug_assert!(min.x < max.x);
        debug_assert!(min.y < max.y);
        debug_assert!(min.z < max.z);

        let mut res = *self;

        if self.x < min.x {
            res.x += (min.x - self.x) * lerp;
        } else if self.x > max.x {
            res.x += (max.x - self.x) * lerp;
        }

        if self.y < min.y {
            res.y += (min.y - self.y) * lerp;
        } else if self.y > max.y {
            res.y += (max.y - self.y) * lerp;
        }

        if self.z < min.z {
            res.z += (min.z - self.z) * lerp;
        } else if self.z > max.z {
            res.z += (max.z - self.z) * lerp;
        }

        res
    }
}
