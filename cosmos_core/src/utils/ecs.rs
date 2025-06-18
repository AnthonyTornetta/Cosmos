//! Bevy ECS utilities

use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

use bevy::prelude::*;

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
            Self::Mut(a) => a.as_ref(),
            Self::Ref(r) => r,
        }
    }
}
impl<T: Component> DerefMut for MutOrMutRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Mut(a) => a.as_mut(),
            Self::Ref(r) => r,
        }
    }
}

/// Bevy's [`RemovedComponents`] doesn't work well w/ fixed updates. This fixes that - just call
/// [`register_fixed_update_removed_component::<T>`] first, then use this in your [`FixedUpdate`] system.
///
/// This gets cleared in [`FixedPostUpdate`] - so make sure to use it before then.
pub type FixedUpdateRemovedComponents<'a, T> = Res<'a, FixedUpdateRemovedComponentsInner<T>>;

/// Bevy's [`RemovedComponents`] doesn't work well w/ fixed updates. This fixes that - just call
/// [`register_removed_component::<T>`] first, then use this in your [`FixedUpdate`] system.
///
/// This gets cleared in [`FixedPostUpdate`] - so make sure to use it before then.
#[derive(Resource, Debug)]
pub struct FixedUpdateRemovedComponentsInner<T: Component>(Vec<Entity>, PhantomData<T>);

impl<T: Component> Default for FixedUpdateRemovedComponentsInner<T> {
    fn default() -> Self {
        Self(vec![], Default::default())
    }
}

impl<T: Component> FixedUpdateRemovedComponentsInner<T> {
    /// Iterates over all removed components.
    pub fn read(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Ensures that this component can be checked for removals in the [`FixedUpdate`] schedule, using
/// [`FixedUpdateRemovedComponents<T>`]
pub fn register_fixed_update_removed_component<T: Component>(app: &mut App) {
    // Prevents duplicate registrations of the same component, which can happen if multiple things
    // rely on this being usable in [`FixedUpdate`].
    if app.world().get_resource::<FixedUpdateRemovedComponentsInner<T>>().is_some() {
        return;
    }

    fn move_to_res<T: Component>(
        mut fixed_update_version: ResMut<FixedUpdateRemovedComponentsInner<T>>,
        mut removed_comps: RemovedComponents<T>,
    ) {
        fixed_update_version
            .as_mut()
            .0
            .append(&mut removed_comps.read().collect::<Vec<_>>());
    }

    fn clear_removed_comps<T: Component>(mut fixed_update_removed: ResMut<FixedUpdateRemovedComponentsInner<T>>) {
        fixed_update_removed.as_mut().0.clear();
    }

    app.add_systems(PostUpdate, move_to_res::<T>)
        .add_systems(FixedPostUpdate, clear_removed_comps::<T>)
        .init_resource::<FixedUpdateRemovedComponentsInner<T>>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, despawn_with_handler);

    app.register_type::<DespawnWith>();
}
