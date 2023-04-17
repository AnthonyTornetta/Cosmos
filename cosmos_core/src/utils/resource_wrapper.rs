//! Used to get around the derive Resource requirement for resources

use bevy::prelude::Resource;

/// Used to get around the derive Resource requirement for resources
#[derive(Resource)]
pub struct ResourceWrapper<T>(pub T);
