use bevy::{
    prelude::{
        App, Commands, Component, DespawnRecursiveExt, Entity, EventReader, EventWriter,
        GlobalTransform, Parent, PbrBundle, Quat, Query, Res, Transform, Vec3, With,
    },
    time::Time,
};
use bevy_rapier3d::{
    prelude::{
        ActiveEvents, ActiveHooks, Ccd, Collider, CollidingEntities, CollisionEvent,
        ContactModificationContextView, LockedAxes, NoUserData, PhysicsHooksWithQuery,
        PhysicsHooksWithQueryResource, QueryFilter, RapierContext, RigidBody, Sensor, SolverFlags,
        TOIStatus, Toi, Velocity,
    },
    rapier::prelude::Real,
};

#[derive(Debug)]
pub struct LaserCollideEvent {
    entity_hit: Entity,
    world_location: Vec3,
    normal: Vec3,
    hit: Entity,
}

/// NEW APPROACH
/// Make bullet sensor
/// Store its previous position as a component
/// update that every frame
///
/// Listen for its collision event
/// When one is found:
/// Make a raycast from its previous position to its current position
/// See what that intersects with, if nothing, then keep the laser alive, if something, then shazam!
/// I don't like this much, but its better than the below method.

/// This doesn't work
// struct MyPhysicsHooks;

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
// impl PhysicsHooksWithQuery<(Option<&NoCollide>, Option<&Parent>)> for MyPhysicsHooks {
//     fn modify_solver_contacts(
//         &self,
//         context: ContactModificationContextView,
//         query: &Query<(Option<&NoCollide>, Option<&Parent>)>,
//     ) {
//         if let Ok((no_collide, _)) = query.get(context.collider1()) {
//             if let Some(no_collide) = no_collide {
//                 if no_collide.fired == context.collider2() {
//                     context.raw.solver_contacts.clear();
//                     println!("CLEARED!!!");
//                 } else {
//                     if let Ok((_, parent)) = query.get(context.collider2()) {
//                         if let Some(parent) = parent {
//                             if no_collide.fired == parent.get() {
//                                 // despite this clearing the contacts, it STILL collides with the ship
//                                 // ?????????????????????????????????????????????????????????????????/
//                                 // I give up on this stupidity for now, I just can't take it anymore.
//                                 context.raw.solver_contacts.clear();
//                                 println!("CLEARED!!!");
//                             } else {
//                                 println!(
//                                     "neq, wanted {}, but got {}",
//                                     no_collide.fired.index(),
//                                     parent.get().index()
//                                 );
//                             }
//                         }
//                     }
//                 }
//             }
//         } else {
//             if let Ok((no_collide, _)) = query.get(context.collider2()) {
//                 if let Some(no_collide) = no_collide {
//                     if no_collide.fired == context.collider1() {
//                         context.raw.solver_contacts.clear();
//                         println!("CLEARED!!!");
//                     } else {
//                         if let Ok((_, parent)) = query.get(context.collider1()) {
//                             if let Some(parent) = parent {
//                                 if no_collide.fired == parent.get() {
//                                     context.raw.solver_contacts.clear();
//                                     println!("CLEARED!!!");
//                                 } else {
//                                     println!(
//                                         "neq, wanted {}, but got {}",
//                                         no_collide.fired.index(),
//                                         parent.get().index()
//                                     );
//                                 }
//                             }
//                         }
//                     }
//                 }
//             } else {
//                 println!("Err ;(");
//             }
//         }
//     }
// }

#[derive(Component)]
/// This is used to prevent the laser from colliding with the entity that fired it
/// If this component is found on the object that it was fired on, then no collision will be registered
pub struct NoCollide {
    laser: Entity,
    fired: Entity,
}

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
}

impl Laser {
    /// Spawns a laser with the given position & velocity
    ///
    /// This takes a PBR that contains mesh data. The transform field will be overwritten
    ///
    /// Base strength is 100
    ///
    pub fn spawn_custom_pbr(
        position: Vec3,
        laser_velocity: Vec3,
        firer_velocity: Vec3,
        _strength: f32,
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
                // strength,
                active: true,
            })
            .insert(pbr)
            .insert(Ccd { enabled: true })
            .insert(RigidBody::Dynamic)
            .insert(LockedAxes::ROTATION_LOCKED)
            .insert(CollidingEntities::default())
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
            ent_cmds.insert(NoCollide {
                fired: ent,
                laser: laser_entity,
            });
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
            Entity,
            Option<&NoCollide>,
            &mut Laser,
            &CollidingEntities,
            &GlobalTransform,
            &Velocity,
            &Collider,
        ),
        With<Laser>,
    >,
    mut commands: Commands,
    mut event_reader: EventReader<CollisionEvent>,
    mut event_writer: EventWriter<LaserCollideEvent>,
    rapier_context: Res<RapierContext>,
) {
    for (
        laser_entity,
        no_collide_entity,
        mut laser,
        collided_with_entities,
        transform,
        velocity,
        collider,
    ) in query.iter_mut()
    {
        if laser.active {
            for collided_with_entity in collided_with_entities.iter() {
                if let Some(no_collide) = no_collide_entity {
                    if no_collide.fired == collided_with_entity && laser_entity == no_collide.laser
                    {
                        continue;
                    }
                }

                if !laser.active {
                    break;
                }

                if let Some((entity, toi)) = rapier_context.cast_shape(
                    transform.translation(),
                    Quat::from_affine3(&transform.affine()),
                    velocity.linvel,
                    collider,
                    1.0.into(),
                    QueryFilter::default(),
                ) {
                    // The second one is being hit, the first one is the laser
                    // (aka norm2 is the one being hit, norm1 is the laser)

                    match toi.status {
                        TOIStatus::Converged => {
                            println!("NORM 1: {}, NORM 2: {}", toi.normal1, toi.normal2);

                            println!("{}", toi.witness2);
                        }
                        TOIStatus::Penetrating => {
                            event_writer.send(LaserCollideEvent {
                                world_location: (),
                                normal: (),
                                hit: (),
                            });
                        }
                        _ => {}
                    }
                }
                // if let Some(contact_pair) =
                //     rapier_context.contact_pair(laser_entity, collided_with_entity)
                // {
                //     println!("Manifolds count: {}", contact_pair.manifolds_len());
                //     // contact_pair.manifold(0).unwrap().find_deepest_contact().unwrap().
                // }

                laser.active = false;
                println!(
                    "BANG! Hit {}! Time to despawn self!",
                    collided_with_entity.index()
                );

                // event_writer.send(LaserCollideEvent { world_location: , normal: (), hit: () })
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

// fn startup_sys(mut commands: Commands) {
//     commands.insert_resource(PhysicsHooksWithQueryResource(Box::new(MyPhysicsHooks)));
// }

pub(crate) fn register(app: &mut App) {
    app.add_system(handle_events)
        .add_system(despawn_lasers)
        .add_event::<LaserCollideEvent>();
    // .add_startup_system(startup_sys);
}
