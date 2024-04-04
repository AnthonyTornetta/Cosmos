//! Custom collision filtering system, based off
//! https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering

use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Query, SystemParam},
    },
    hierarchy::Parent,
};
use bevy_rapier3d::{
    geometry::SolverFlags,
    pipeline::{BevyPhysicsHooks, PairFilterContextView},
};

/// Indicates that this entity cannot be collided with.
/// This should be used with [`CannotCollideWith`]
pub struct CannotCollideWithEntity {
    /// The entity to not collide with
    pub entity: Entity,
    /// If this is true, when an entity is hit the parent(s) of that
    /// entity will be searched and checked if they match this entity.
    ///
    /// This is mostly useful when dealing with structures, because they will never
    /// be directly collided with, but rather their chunk children.
    pub search_parents: bool,
}

#[derive(Component)]
/// If this is on an entity, then this entity will not collide with any entities
/// present in the list of [`CannotCollideWithEntity`]s.
///
/// This doesn't work for collisions with sensors though, so keep that in mind.
///
/// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
/// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
pub struct CannotCollideWith(Vec<CannotCollideWithEntity>);

impl CannotCollideWith {
    /// A convenience method to initialize this with just one cannot collide with entity.
    ///
    /// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
    /// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
    pub fn single(cannot_collide_with_entity: CannotCollideWithEntity) -> Self {
        Self::new(vec![cannot_collide_with_entity])
    }

    /// This entity will not collidew with any of the entities provided.
    ///
    /// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
    /// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
    pub fn new(cannot_collide_with_enties: Vec<CannotCollideWithEntity>) -> Self {
        Self(cannot_collide_with_enties)
    }
}

// From: https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering

/// A custom filter that allows filters collisions between rigid-bodies
/// with the CannotCollideWith component.
#[derive(SystemParam)]
pub struct CosmosPhysicsFilter<'w, 's> {
    q_cannot_collide_with: Query<'w, 's, &'static CannotCollideWith>,
    q_parent: Query<'w, 's, &'static Parent>,
}

impl<'w, 's> CosmosPhysicsFilter<'w, 's> {
    fn check_pair_filter(&self, context: PairFilterContextView) -> bool {
        if let Ok(cannot_collide_with) = self.q_cannot_collide_with.get(context.collider1()) {
            if !check_should_collide(context.collider2(), &self.q_parent, cannot_collide_with) {
                return false;
            }
        }

        if let Ok(cannot_collide_with) = self.q_cannot_collide_with.get(context.collider2()) {
            if !check_should_collide(context.collider1(), &self.q_parent, cannot_collide_with) {
                return false;
            }
        }

        true
    }
}

fn check_should_collide(mut entity_checking: Entity, q_parent: &Query<&Parent>, cannot_collide_with: &CannotCollideWith) -> bool {
    cannot_collide_with.0.iter().any(|x| {
        if x.entity == entity_checking {
            return false;
        }

        if !x.search_parents {
            return true;
        }

        while let Ok(check_next) = q_parent.get(entity_checking) {
            entity_checking = check_next.get();
            if x.entity == entity_checking {
                return false;
            }
        }

        true
    })
}

impl BevyPhysicsHooks for CosmosPhysicsFilter<'_, '_> {
    fn filter_contact_pair(&self, context: PairFilterContextView) -> Option<SolverFlags> {
        if self.check_pair_filter(context) {
            Some(SolverFlags::COMPUTE_IMPULSES)
        } else {
            None
        }
    }

    fn filter_intersection_pair(&self, context: PairFilterContextView) -> bool {
        self.check_pair_filter(context)
    }
}
