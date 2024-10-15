//! Custom collision filtering system, based off
//! https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering

use bevy::{
    app::App,
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
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, IdentifiableComponent, SyncableComponent};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
/// Indicates that this entity cannot be collided with.
/// This should be used with [`CollisionBlacklist`]
pub struct CollisionBlacklistedEntity {
    /// The entity to not collide with
    pub entity: Entity,
    /// If this is true, when an entity is hit the parent(s) of that
    /// entity will be searched and checked if they match this entity.
    ///
    /// This is mostly useful when dealing with structures, because they will never
    /// be directly collided with, but rather their chunk children.
    pub search_parents: bool,
}

#[derive(Component, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// If this is on an entity, then this entity will not collide with any entities
/// present in the list of [`CollisionBlacklistedEntity`]s.
///
/// This doesn't work for collisions with sensors though, so keep that in mind.
///
/// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
/// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
pub struct CollisionBlacklist(Vec<CollisionBlacklistedEntity>);

impl CollisionBlacklist {
    /// A convenience method to initialize this with just one cannot collide with entity.
    ///
    /// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
    /// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
    pub fn single(blacklist_entity: CollisionBlacklistedEntity) -> Self {
        Self::new(vec![blacklist_entity])
    }

    /// This entity will not collidew with any of the entities provided.
    ///
    /// # DON'T FORGET TO ADD THE `ActiveHooks::FILTER_CONTACT_PAIRS` COMPONENT FROM `bevy_rapier`!
    /// See https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering for more.
    pub fn new(blacklist_entity: Vec<CollisionBlacklistedEntity>) -> Self {
        Self(blacklist_entity)
    }

    /// Checks if this entity should be collided with.
    pub fn check_should_collide(&self, mut entity_checking: Entity, q_parent: &Query<&Parent>) -> bool {
        self.0.iter().any(|x| {
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
}

impl IdentifiableComponent for CollisionBlacklist {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:collision_blacklist"
    }
}

impl SyncableComponent for CollisionBlacklist {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        Some(Self(
            self.0
                .into_iter()
                .flat_map(|x| {
                    Some(CollisionBlacklistedEntity {
                        search_parents: x.search_parents,
                        entity: mapping.client_from_server(&x.entity)?,
                    })
                })
                .collect::<Vec<CollisionBlacklistedEntity>>(),
        ))
    }
}

// From: https://rapier.rs/docs/user_guides/bevy_plugin/advanced_collision_detection/#contact-and-intersection-filtering

/// A custom filter that allows filters collisions between rigid-bodies
/// with the [`CollisionBlacklist`] component.
#[derive(SystemParam)]
pub struct CosmosPhysicsFilter<'w, 's> {
    q_collision_blacklist: Query<'w, 's, &'static CollisionBlacklist>,
    q_parent: Query<'w, 's, &'static Parent>,
}

impl CosmosPhysicsFilter<'_, '_> {
    fn check_pair_filter(&self, context: PairFilterContextView) -> bool {
        if let Ok(collision_blacklist) = self.q_collision_blacklist.get(context.collider1()) {
            if !collision_blacklist.check_should_collide(context.collider2(), &self.q_parent) {
                return false;
            }
        }

        if let Ok(collision_blacklist) = self.q_collision_blacklist.get(context.collider2()) {
            if !collision_blacklist.check_should_collide(context.collider1(), &self.q_parent) {
                return false;
            }
        }

        true
    }
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

pub(super) fn register(app: &mut App) {
    sync_component::<CollisionBlacklist>(app);
}
