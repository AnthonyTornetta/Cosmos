//! Provides some useful bevy bundles

use bevy::prelude::*;

use crate::physics::location::Location;

/// A component bundle for PBR entities with a [`Mesh`] and a [`StandardMaterial`].
pub type CosmosPbrBundle = CosmosMaterialMeshBundle<StandardMaterial>;

#[derive(Debug, Component, Reflect, Default, Clone, Copy)]
/// A quaternion representing the starting rotation
///
/// This will be removed as soon as the transform is constructed
pub struct BundleStartingRotation(pub Quat);

impl From<Quat> for BundleStartingRotation {
    fn from(value: Quat) -> Self {
        Self(value)
    }
}

/// A component bundle for entities with a [`Mesh`] and a [`Material`].
///
/// A [`Transform`] component will be added, then immediately set values based on the [`Location`]
/// and [`BundleStartingRotation`].
#[derive(Bundle, Clone, Debug, Reflect, Default)]
pub struct CosmosMaterialMeshBundle<M: Material> {
    /// The bevy mesh
    pub mesh: Mesh3d,
    /// The material type provided
    pub material: MeshMaterial3d<M>,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// The location of this entity
    pub location: Location,
    /// The rotation of this entity
    pub rotation: BundleStartingRotation,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}
