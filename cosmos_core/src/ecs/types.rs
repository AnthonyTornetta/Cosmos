//! Useful types for interacting with the ecs

use std::ops::{Deref, DerefMut};

use bevy::ecs::{
    component::{Component, Mutable},
    world::Mut,
};

/// A piece of data that may be owned, or may be a bevy [`Mut`] reference.
///
/// Useful when working with data that could be already on the entity, or needs to be attached once
/// done doing work on this data.
pub enum OwnedOrMut<'w, T: Component<Mutability = Mutable>> {
    /// This data is owned
    Owned(T),
    /// This is a bevy [`Mut`] reference
    Mut(Mut<'w, T>),
}

impl<'w, T: Component<Mutability = Mutable>> OwnedOrMut<'w, T> {
    /// If this is the owned version, returns Some(T)
    pub fn owned(self) -> Option<T> {
        match self {
            Self::Owned(t) => Some(t),
            Self::Mut(_) => None,
        }
    }
}

impl<'w, T: Component<Mutability = Mutable>> Deref for OwnedOrMut<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(t) => t,
            Self::Mut(t) => t.as_ref(),
        }
    }
}

impl<'w, T: Component<Mutability = Mutable>> DerefMut for OwnedOrMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(t) => t,
            Self::Mut(t) => t.as_mut(),
        }
    }
}
