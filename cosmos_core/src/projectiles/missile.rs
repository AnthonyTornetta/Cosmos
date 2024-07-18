//! A missile is something that flies in a straight line & may collide with a block, causing
//! it to take damage.

use std::time::Duration;

use bevy::{
    color::Color,
    core::Name,
    ecs::{
        query::Added,
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    },
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{App, Commands, Component, Entity, Query, Update},
    reflect::Reflect,
};
use bevy_rapier3d::{
    geometry::{ActiveEvents, ActiveHooks, Collider},
    prelude::RigidBody,
};
use serde::{Deserialize, Serialize};

use crate::{
    netty::{
        sync::{sync_component, ComponentSyncingSet, IdentifiableComponent, SyncableComponent},
        system_sets::NetworkingSystemsSet,
    },
    physics::location::{CosmosBundleSet, LocationPhysicsSet},
};

#[derive(Component, Debug, Serialize, Deserialize, Clone, PartialEq)]
/// A missile is something that flies in a straight line & may collide with a block, causing
/// it to take damage.
pub struct Missile {
    /// The strength of this missile, used to calculate block damage
    pub strength: f32,

    /// How long the missile can be alive before exploding
    pub lifetime: Duration,

    /// Color of the missile's explosion, if it has one specified
    pub color: Option<Color>,
}

impl IdentifiableComponent for Missile {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:missile"
    }
}

impl SyncableComponent for Missile {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

fn on_add_missile(q_added_missile: Query<Entity, Added<Missile>>, mut commands: Commands) {
    for missile_ent in q_added_missile.iter() {
        commands.entity(missile_ent).insert((
            Name::new("Missile"),
            RigidBody::Dynamic,
            Collider::cuboid(0.15, 0.15, 0.5),
            NotShadowCaster,
            ActiveEvents::COLLISION_EVENTS,
            ActiveHooks::FILTER_CONTACT_PAIRS,
            NotShadowReceiver,
        ));
    }
}

#[derive(Component, Debug, Reflect, Clone, Copy, Serialize, Deserialize, PartialEq)]
/// Something that will cause damage to nearby entities that it hits.
pub struct Explosion {
    /// The power of the explosion is used to calculate its radius & effectiveness against blocks.
    ///
    /// The radius of an explosion (assuming no blocks to dampen its power) is calculated as `sqrt(power)`.
    pub power: f32,

    /// The color the explosion should be
    pub color: Option<Color>,
}

impl IdentifiableComponent for Explosion {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:explosion"
    }
}

impl SyncableComponent for Explosion {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System used for dealing with explosions that happen in the world.
///
/// Put anything that creates an explosion before [`ExplosionSystemSet::ProcessExplosions`].
pub enum ExplosionSystemSet {
    /// Put anything that creates an explosion before [`ExplosionSystemSet::PreProcessExplosions`].
    ///
    /// This set exists to give the cosmos location system time to assign the explosion a world & transform.
    PreProcessExplosions,
    /// Put anything that creates an explosion before [`ExplosionSystemSet::PreProcessExplosions`].
    ///
    /// In this set, explosions will cause damage to things they are near
    ProcessExplosions,
}

pub(super) fn register(app: &mut App) {
    sync_component::<Missile>(app);
    sync_component::<Explosion>(app);

    #[cfg(feature = "server")]
    {
        // Setup explosion before they are synced to clients
        app.configure_sets(
            Update,
            (
                ExplosionSystemSet::PreProcessExplosions
                    .before(LocationPhysicsSet::DoPhysics)
                    .before(CosmosBundleSet::HandleCosmosBundles),
                ExplosionSystemSet::ProcessExplosions
                    .after(LocationPhysicsSet::DoPhysics)
                    .after(CosmosBundleSet::HandleCosmosBundles),
            )
                .chain(),
        );
    }
    #[cfg(feature = "client")]
    {
        // Receive explosions from server before processing them
        app.configure_sets(
            Update,
            (ExplosionSystemSet::PreProcessExplosions, ExplosionSystemSet::ProcessExplosions)
                .after(LocationPhysicsSet::DoPhysics)
                .after(NetworkingSystemsSet::SyncComponents)
                .after(CosmosBundleSet::HandleCosmosBundles)
                .chain(),
        );
    }

    #[cfg(feature = "client")]
    app.add_systems(Update, on_add_missile.in_set(ComponentSyncingSet::PostComponentSyncing));
    #[cfg(feature = "server")]
    app.add_systems(Update, on_add_missile.in_set(ComponentSyncingSet::PreComponentSyncing));

    app.register_type::<Explosion>();
}
