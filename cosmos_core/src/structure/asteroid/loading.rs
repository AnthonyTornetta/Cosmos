use bevy::prelude::Component;

#[derive(Debug, Component)]
/// This is put onto asteroids that need to be generated, but is not present
/// while they are being generated.
pub struct AsteroidNeedsCreated;
