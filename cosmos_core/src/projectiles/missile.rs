//! A missile is something that flies in a straight line & may collide with a block, causing
//! it to take damage. Use `Missile::spawn` to create a missile.

use std::time::Duration;

use bevy::{
    core::Name,
    ecs::{
        query::Added,
        schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
        system::EntityCommands,
    },
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::{App, Commands, Component, Entity, Event, Query, Transform, Update, Vec3},
    reflect::Reflect,
    render::color::Color,
};
use bevy_rapier3d::{
    geometry::{ActiveEvents, ActiveHooks, Collider},
    prelude::{PhysicsWorld, RigidBody, Velocity, WorldId},
};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::bundles::CosmosPbrBundle,
    netty::sync::{sync_component, ComponentSyncingSet, SyncableComponent},
    persistence::LoadingDistance,
    physics::{
        collision_handling::{CollisionBlacklist, CollisionBlacklistedEntity},
        location::{CosmosBundleSet, Location, LocationPhysicsSet},
    },
};

#[derive(Debug, Event)]
/// The entity hit represents the entity hit by the missile
///
/// The world location the exact position the world this collision happened
///
/// The relative location is based off the hit entity's world view
/// - The relative location is how that object would perceive the point based on how it views the world
/// - This means that the relative counts the hit entity's rotation
/// - To get the world point (assuming this entity hasn't moved), simply do
///     - (That entity's rotation quaternion * relative_location) + that entity's global transform position.
pub struct MissileCollideEvent {
    entity_hit: Entity,
    local_position_hit: Vec3,
    missile_strength: f32,
}

impl MissileCollideEvent {
    /// Gets the entity this missile hit
    ///
    /// *NOTE*: Make sure to verify this entity still exists before processing it
    pub fn entity_hit(&self) -> Entity {
        self.entity_hit
    }

    /// The explosive strength of this missile
    pub fn missile_strength(&self) -> f32 {
        self.missile_strength
    }

    /// The location this missile hit relative to the entity it hit's transform.
    pub fn local_position_hit(&self) -> Vec3 {
        self.local_position_hit
    }
}

#[derive(Component, Serialize, Deserialize, Clone)]
/// A missile is something that flies in a straight line & may collide with a block, causing
/// it to take damage. Use `Missile::spawn` to create a missile.
pub struct Missile {
    /// The strength of this missile, used to calculate block damage
    pub strength: f32,

    /// How long the missile can be alive before exploding
    pub lifetime: Duration,

    /// Color of the missile's explosion, if it has one specified
    pub color: Option<Color>,
}

impl SyncableComponent for Missile {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:missile"
    }

    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl Missile {
    /// Spawns a missile with the given position & velocity
    ///
    /// * `missile_velocity` - The missile's velocity. Do not add the parent's velocity for this, use `firer_velocity` instead.
    /// * `firer_velocity` - The missile's parent's velocity.
    pub fn spawn<'a>(
        position: Location,
        color: Option<Color>,
        missile_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        world_id: WorldId,
        commands: &'a mut Commands,
        missile_lifetime: Duration,
    ) -> EntityCommands<'a> {
        let pbr = CosmosPbrBundle {
            rotation: Transform::from_xyz(0.0, 0.0, 0.0)
                .looking_at(missile_velocity, Vec3::Y)
                .rotation
                .into(),
            location: position,
            ..Default::default()
        };

        let mut ent_cmds = commands.spawn((
            Missile {
                color,
                strength,
                lifetime: missile_lifetime,
            },
            pbr,
            Velocity {
                linvel: missile_velocity + firer_velocity,
                ..Default::default()
            },
            PhysicsWorld { world_id },
            LoadingDistance::new(1, 2),
        ));

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(CollisionBlacklist::single(CollisionBlacklistedEntity {
                entity: ent,
                search_parents: true,
            }));
        }

        ent_cmds
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

#[derive(Component, Reflect, Clone, Serialize, Deserialize)]
/// Something that will cause damage to nearby entities that it hits.
pub struct Explosion {
    /// The power of the explosion is used to calculate its radius & effectiveness against blocks.
    ///
    /// The radius of an explosion (assuming no blocks to dampen its power) is calculated as `sqrt(power)`.
    pub power: f32,

    /// The color the explosion should be
    pub color: Option<Color>,
}

impl SyncableComponent for Explosion {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:explosion"
    }

    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// System used for dealing with explosions that happen in the world.
///
/// Put anything that creates an explosion before [`ExplosionSystemSet::ProcessExplosions`].
pub enum ExplosionSystemSet {
    /// Put anything that creates an explosion before [`ExplosionSystemSet::ProcessExplosions`].
    ///
    /// In this set, explosions will cause damage to things they are near
    ProcessExplosions,
}

pub(super) fn register(app: &mut App) {
    sync_component::<Missile>(app);
    sync_component::<Explosion>(app);

    let mut sets = ExplosionSystemSet::ProcessExplosions
        .before(LocationPhysicsSet::DoPhysics)
        .before(CosmosBundleSet::HandleCosmosBundles);

    #[cfg(feature = "server")]
    {
        // Setup explosion before they are synced to clients
        sets = sets.before(ComponentSyncingSet::PreComponentSyncing);
    }
    #[cfg(feature = "client")]
    {
        // Receive explosions from server before processing them
        sets = sets.after(ComponentSyncingSet::PostComponentSyncing);
    }

    app.configure_sets(Update, sets);

    #[cfg(feature = "client")]
    app.add_systems(Update, on_add_missile.in_set(ComponentSyncingSet::PostComponentSyncing));
    #[cfg(feature = "server")]
    app.add_systems(Update, on_add_missile.in_set(ComponentSyncingSet::PreComponentSyncing));

    // app.add_systems(
    //     Update,
    //     (respond_to_collisions, despawn_missiles)
    //         .before(ExplosionSystemSet::ProcessExplosions)
    //         .chain(),
    // )
    app.add_event::<MissileCollideEvent>().register_type::<Explosion>();
}
