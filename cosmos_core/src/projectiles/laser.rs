use bevy::{
    prelude::{
        App, Commands, Component, DespawnRecursiveExt, Entity, EventWriter, GlobalTransform,
        Parent, PbrBundle, Quat, Query, Res, Transform, Vec3, With,
    },
    time::Time,
};
use bevy_rapier3d::prelude::{
    ActiveEvents, ActiveHooks, Collider, ContactModificationContextView, LockedAxes,
    PhysicsHooksWithQuery, PhysicsHooksWithQueryResource, QueryFilter, RapierContext, RigidBody,
    Sensor, Velocity,
};

#[derive(Debug)]
/// The entity hit represents the entity hit by the laser
///
/// The world location the exact position the world this collision happened
///
/// The relative location is based off the hit entity's world view
/// - The relative location is how that object would perceive the point based on how it views the world
/// - This means that the relative counts the hit entity's rotation
/// - To get the world point (assuming this entity hasn't moved), simply do
///     - (That entity's rotation quaternion * relative_location) + that entity's global transform position.
pub struct LaserCollideEvent {
    entity_hit: Entity,
    local_position_hit: Vec3,
    laser_strength: f32,
}

impl LaserCollideEvent {
    pub fn entity_hit(&self) -> Entity {
        self.entity_hit
    }

    pub fn laser_strength(&self) -> f32 {
        self.laser_strength
    }

    pub fn local_position_hit(&self) -> Vec3 {
        self.local_position_hit
    }
}

struct MyPhysicsHooks;

/// HEY! IF YOU CHANGE THE DATA IN THE <>, MAKE SURE TO CHANGE IT IN THE COSMOS_CORE_PLUGIN.RS FILE TOO!
/// It's the data in the rapier plugin.
/// But why would I have to do such a silly thing?
/// Who thought that was a good idea?
/// I just don't know.
/// But I do know, if you forget it, you won't get an error.
/// In fact, you won't get any text at all.
/// It will just silently do nothing, and make you sad.
///  
/// despite this clearing the contacts, it STILL collides with the ship
/// ?????????????????????????????????????????????????????????????????/
/// I give up on this stupidity for now, I just can't take it anymore.
impl PhysicsHooksWithQuery<(Option<&NoCollide>, Option<&Parent>)> for MyPhysicsHooks {
    fn modify_solver_contacts(
        &self,
        context: ContactModificationContextView,
        query: &Query<(Option<&NoCollide>, Option<&Parent>)>,
    ) {
        if let Ok((no_collide, _)) = query.get(context.collider1()) {
            if let Some(no_collide) = no_collide {
                if no_collide.0 == context.collider2() {
                    context.raw.solver_contacts.clear();
                } else {
                    if let Ok((_, parent)) = query.get(context.collider2()) {
                        if let Some(parent) = parent {
                            if no_collide.0 == parent.get() {
                                // despite this clearing the contacts, it STILL collides with the ship
                                // ?????????????????????????????????????????????????????????????????/
                                // I give up on this stupidity for now, I just can't take it anymore.
                                context.raw.solver_contacts.clear();
                            }
                        }
                    }
                }
            }
        } else {
            if let Ok((no_collide, _)) = query.get(context.collider2()) {
                if let Some(no_collide) = no_collide {
                    if no_collide.0 == context.collider1() {
                        context.raw.solver_contacts.clear();
                    } else {
                        if let Ok((_, parent)) = query.get(context.collider1()) {
                            if let Some(parent) = parent {
                                if no_collide.0 == parent.get() {
                                    context.raw.solver_contacts.clear();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Component)]
/// This is used to prevent the laser from colliding with the entity that fired it
/// If this component is found on the object that it was fired on, then no collision will be registered
pub struct NoCollide(Entity);

#[derive(Component)]
struct FireTime {
    time: f32,
}

#[derive(Component)]
pub struct Laser {
    // strength: f32,
    /// commands despawning entity isn't instant, but changing this field is.
    /// Thus, this field should always be checked when determining if a laser should break/damage something.
    active: bool,
    pub strength: f32,
}

impl Laser {
    /// Spawns a laser with the given position & velocity
    ///
    /// This takes a PBR that contains mesh data. The transform field will be overwritten
    ///
    /// Base strength is 100?
    ///
    pub fn spawn_custom_pbr(
        position: Vec3,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        mut pbr: PbrBundle,
        time: &Time,
        commands: &mut Commands,
    ) -> Entity {
        pbr.transform = Transform {
            translation: position,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        };

        pbr.transform.look_at(position + laser_velocity, Vec3::Y);

        let mut ent_cmds = commands.spawn_empty();

        let laser_entity = ent_cmds.id();

        ent_cmds
            .insert(Laser {
                strength,
                active: true,
            })
            .insert(pbr)
            .insert(RigidBody::Dynamic)
            .insert(LockedAxes::ROTATION_LOCKED)
            .insert(Collider::cuboid(0.05, 0.05, 1.0))
            .insert(Velocity {
                linvel: laser_velocity + firer_velocity,
                ..Default::default()
            })
            .insert(FireTime {
                time: time.elapsed_seconds(),
            })
            .insert(ActiveEvents::COLLISION_EVENTS)
            .insert(ActiveHooks::MODIFY_SOLVER_CONTACTS)
            .insert(Sensor);

        if let Some(ent) = no_collide_entity {
            ent_cmds.insert(NoCollide(ent));
        }

        laser_entity
    }

    /// Spawns a laser with the given position & velocity
    /// Base strength is 100
    ///
    pub fn spawn(
        position: Vec3,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        strength: f32,
        no_collide_entity: Option<Entity>,
        time: &Time,
        commands: &mut Commands,
    ) -> Entity {
        Self::spawn_custom_pbr(
            position,
            laser_velocity,
            firer_velocity,
            strength,
            no_collide_entity,
            PbrBundle {
                ..Default::default()
            },
            time,
            commands,
        )
    }
}

fn handle_events(
    mut query: Query<
        (
            &GlobalTransform,
            Entity,
            Option<&NoCollide>,
            &mut Laser,
            &Velocity,
            &Collider,
        ),
        With<Laser>,
    >,
    mut commands: Commands,
    // mut event_reader: EventReader<CollisionEvent>,
    mut event_writer: EventWriter<LaserCollideEvent>,
    rapier_context: Res<RapierContext>,
    parent_query: Query<&Parent>,
) {
    for (transform, laser_entity, no_collide_entity, mut laser, velocity, collider) in
        query.iter_mut()
    {
        if laser.active {
            if let Some((entity, toi)) = rapier_context.cast_shape(
                transform.translation(),
                Quat::from_affine3(&transform.affine()),
                velocity.linvel,
                collider,
                velocity.linvel.dot(velocity.linvel),
                QueryFilter::predicate(QueryFilter::default(), &|entity| {
                    if let Some(no_collide_entity) = no_collide_entity {
                        if no_collide_entity.0 == entity {
                            false
                        } else {
                            if let Ok(parent) = parent_query.get(entity) {
                                if parent.get() == no_collide_entity.0 {
                                    false
                                } else {
                                    true
                                }
                            } else {
                                false
                            }
                        }
                    } else {
                        true
                    }
                }),
            ) {
                println!("HIT {} @ {}", entity.index(), toi.witness1);

                event_writer.send(LaserCollideEvent {
                    entity_hit: entity,
                    local_position_hit: toi.witness1 + velocity.linvel.normalize() * 0.01,
                    laser_strength: laser.strength,
                });

                laser.active = false;
                commands.entity(laser_entity).despawn_recursive();
            }
        }
    }
}

fn despawn_lasers(
    mut commands: Commands,
    query: Query<(Entity, &FireTime), With<Laser>>,
    time: Res<Time>,
) {
    for (ent, fire_time) in query.iter() {
        if time.elapsed_seconds() - fire_time.time > 5.0 {
            commands.entity(ent).despawn_recursive();
        }
    }
}

fn startup_sys(mut commands: Commands) {
    commands.insert_resource(PhysicsHooksWithQueryResource(Box::new(MyPhysicsHooks)));
}

pub(crate) fn register(app: &mut App) {
    app.add_system(handle_events)
        .add_system(despawn_lasers)
        .add_event::<LaserCollideEvent>()
        .add_startup_system(startup_sys);
}
