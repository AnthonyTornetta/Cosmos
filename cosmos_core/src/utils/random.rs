//! Utility functions for random number generation

/// Computes a random value within the given range
pub fn random_range(low: f32, high: f32) -> f32 {
    rand::random::<f32>() * (high - low) + low
}
