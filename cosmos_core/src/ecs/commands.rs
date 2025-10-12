use std::ops::{Deref, DerefMut};

use bevy::ecs::{
    component::{Component, Mutable},
    entity::Entity,
    system::{Commands, Query},
    world::Mut,
};

pub enum OwnedOrMut<'w, T: Component<Mutability = Mutable>> {
    Owned(T),
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
