//! Bevy ECS utilities

use std::ops::Deref;
use std::ops::DerefMut;

use bevy::app::PostUpdate;
use bevy::prelude::App;
use bevy::prelude::Commands;
use bevy::prelude::Component;
use bevy::prelude::Entity;
use bevy::prelude::Mut;
use bevy::prelude::Query;
use bevy::prelude::With;
use bevy::prelude::Without;
use bevy::reflect::Reflect;

use crate::ecs::NeedsDespawned;

/// When the entity referenced doesn't exist, then the entity this is attached to will be flagged
/// for deletion
#[derive(Component, Reflect, Debug)]
pub struct DespawnWith(pub Entity);

fn despawn_with_handler(
    mut commands: Commands,
    q_entity: Query<Entity, With<NeedsDespawned>>,
    q_despawn_with: Query<(Entity, &DespawnWith), Without<NeedsDespawned>>,
) {
    for (ent, despawn_with) in q_despawn_with.iter() {
        if q_entity.contains(despawn_with.0) {
            commands.entity(ent).insert(NeedsDespawned);
        }
    }
}

/// Handles the case where you either have a bevy Mut<T> or a &mut T, and you want to handle both
/// cases.
pub enum MutOrMutRef<'a, T: Component> {
    /// Bevy's Mut<T>
    Mut(Mut<'a, T>),
    /// &mut T
    Ref(&'a mut T),
}

impl<'a, T: Component> From<Mut<'a, T>> for MutOrMutRef<'a, T> {
    fn from(value: Mut<'a, T>) -> Self {
        Self::Mut(value)
    }
}

impl<'a, T: Component> From<&'a mut T> for MutOrMutRef<'a, T> {
    fn from(value: &'a mut T) -> Self {
        Self::Ref(value)
    }
}

impl<T: Component> Deref for MutOrMutRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Mut(ref a) => a.as_ref(),
            Self::Ref(r) => r,
        }
    }
}
impl<T: Component> DerefMut for MutOrMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Mut(ref mut a) => a.as_mut(),
            Self::Ref(r) => r,
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, despawn_with_handler);

    app.register_type::<DespawnWith>();
}
