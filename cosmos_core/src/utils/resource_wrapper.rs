//! Used to get around the derive Resource requirement for resources

use bevy::prelude::{Deref, Resource};

/// Used to get around the derive Resource requirement for resources
#[derive(Resource, Deref)]
pub struct ResourceWrapper<T>(pub T);
