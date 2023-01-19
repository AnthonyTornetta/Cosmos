use std::ops::Mul;

use bevy::{
    prelude::{
        App, Commands, Component, DespawnRecursiveExt, Entity, EventReader, EventWriter,
        GlobalTransform, Parent, PbrBundle, Quat, Query, Res, Transform, Vec3, With,
    },
    time::Time,
};
use bevy_rapier3d::prelude::{
    ActiveEvents, ActiveHooks, Ccd, Collider, CollidingEntities, CollisionEvent,
    ContactModificationContextView, LockedAxes, PhysicsHooksWithQuery,
    PhysicsHooksWithQueryResource, RapierContext, RigidBody, Sensor, SolverFlags, TOIStatus, Toi,
    Velocity,
};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, Structure},
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
                if no_collide.fired == context.collider2() {
                    context.raw.solver_contacts.clear();
                    println!("CLEARED!!!");
                } else {
                    if let Ok((_, parent)) = query.get(context.collider2()) {
                        if let Some(parent) = parent {
                            if no_collide.fired == parent.get() {
                                // despite this clearing the contacts, it STILL collides with the ship
                                // ?????????????????????????????????????????????????????????????????/
                                // I give up on this stupidity for now, I just can't take it anymore.
                                context.raw.solver_contacts.clear();
                                println!("CLEARED!!!");
                            } else {
                                println!(
                                    "neq, wanted {}, but got {}",
                                    no_collide.fired.index(),
                                    parent.get().index()
                                );
                            }
                        }
                    }
                }
            }
        } else {
            if let Ok((no_collide, _)) = query.get(context.collider2()) {
                if let Some(no_collide) = no_collide {
                    if no_collide.fired == context.collider1() {
                        context.raw.solver_contacts.clear();
                        println!("CLEARED!!!");
                    } else {
                        if let Ok((_, parent)) = query.get(context.collider1()) {
                            if let Some(parent) = parent {
                                if no_collide.fired == parent.get() {
                                    context.raw.solver_contacts.clear();
                                    println!("CLEARED!!!");
                                } else {
                                    println!(
                                        "neq, wanted {}, but got {}",
                                        no_collide.fired.index(),
                                        parent.get().index()
                                    );
                                }
                            }
                        }
                    }
                }
            } else {
                println!("Err ;(");
            }
        }
    }
}

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
            // .insert(Sensor)
            ;

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
            &Velocity,
        ),
        With<Laser>,
    >,
    mut commands: Commands,
    // mut event_reader: EventReader<CollisionEvent>,
    mut event_writer: EventWriter<LaserCollideEvent>,
    rapier_context: Res<RapierContext>,
) {
    for (laser_entity, no_collide_entity, mut laser, collided_with_entities, velocity) in
        query.iter_mut()
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

                for contact_pair in rapier_context.contacts_with(laser_entity) {
                    if let Some((_, y)) = contact_pair.find_deepest_contact() {
                        let (entity_hit, mut local_position_hit) =
                            if contact_pair.collider1() == laser_entity {
                                (contact_pair.collider2(), y.local_p2())
                            } else {
                                (contact_pair.collider1(), y.local_p1())
                            };

                        // This ensures that it's actually in the block and not 0.00001 above it or something stupid
                        local_position_hit += velocity.linvel.normalize() * 0.01;

                        // Verify this is a valid collision, sometimes it returns NaN for invalid ones
                        if local_position_hit.is_finite() {
                            event_writer.send(LaserCollideEvent {
                                entity_hit,
                                local_position_hit,
                            });

                            laser.active = false;
                            commands.entity(laser_entity).despawn_recursive();
                            break;
                        }
                    }
                }

                // let transform = world_pos_query
                //     .get(laser_entity)
                //     .expect("Every entity that collided must have a GlobalTransform");

                // if let Some((entity_hit, toi)) = rapier_context.cast_shape(
                //     transform.translation(),
                //     Quat::from_affine3(&transform.affine()),
                //     velocity.linvel,
                //     collider,
                //     1.0.into(),
                //     QueryFilter::default(),
                // ) {
                //     // The second one is being hit, the first one is the laser
                //     // (aka norm2 is the one being hit, norm1 is the laser)

                //     match toi.status {
                //         TOIStatus::Converged | TOIStatus::Penetrating => {
                //             let trans = world_pos_query
                //                 .get(entity_hit)
                //                 .expect("Every entity that has collision has a global transform");

                //             let relative_location = Quat::from_affine3(&transform.affine())
                //                 .inverse()
                //                 .mul_vec3(transform.translation() - trans.translation());

                //             event_writer.send(LaserCollideEvent {
                //                 entity_hit,
                //                 world_location: toi.witness2,
                //                 relative_location,
                //             });

                //             laser.active = false;
                //             println!(
                //                 "BANG! Hit {}! Time to despawn self!",
                //                 collided_with_entity.index()
                //             );
                //             commands.entity(laser_entity).despawn_recursive();
                //         }
                //         _ => {}
                //     }
                // }
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

fn respond_event(
    mut reader: EventReader<LaserCollideEvent>,
    parent_query: Query<&Parent>,
    mut structure_query: Query<&mut Structure>,
    blocks: Option<Res<Registry<Block>>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    if let Some(blocks) = blocks {
        for ev in reader.iter() {
            if let Ok(parent) = parent_query.get(ev.entity_hit) {
                if let Ok(mut structure) = structure_query.get_mut(parent.get()) {
                    println!("Hit structure @ {}!", ev.local_position_hit);
                    if let Some(chunk) = structure.chunk_from_entity(&ev.entity_hit) {
                        let chunk_block_coords = (
                            (ev.local_position_hit.x + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
                            (ev.local_position_hit.y + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
                            (ev.local_position_hit.z + CHUNK_DIMENSIONS as f32 / 2.0) as usize,
                        );

                        let (bx, by, bz) = structure
                            .block_coords_for_chunk_block_coords(chunk, chunk_block_coords);

                        println!("HIT {bx}, {by}, {bz} block coords of structure!");

                        if structure.is_within_blocks(bx, by, bz) {
                            structure.set_block_at(
                                bx,
                                by,
                                bz,
                                blocks.from_id("cosmos:grass").unwrap(),
                                &blocks,
                                Some(&mut event_writer),
                            );
                        } else {
                            println!("Bad laser ;(");
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(handle_events)
        .add_system(despawn_lasers)
        .add_system(respond_event)
        .add_event::<LaserCollideEvent>()
        .add_startup_system(startup_sys);
}
