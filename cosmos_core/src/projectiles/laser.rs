use bevy::prelude::{
    App, Commands, Component, Entity, PbrBundle, Quat, Query, Transform, Vec3, With,
};
use bevy_rapier3d::prelude::{
    Ccd, Collider, CollidingEntities, LockedAxes, RigidBody, Sensor, Velocity,
};

#[derive(Component)]
/// This is used to prevent the laser from colliding with the entity that fired it
/// If this component is found on the object that it was fired on, then no collision will be registered
pub struct NoCollide {
    laser: Entity,
    fired: Entity,
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
            .insert(Sensor)
            .insert(RigidBody::Dynamic)
            .insert(LockedAxes::ROTATION_LOCKED)
            .insert(CollidingEntities::default())
            .insert(Collider::cuboid(0.05, 0.05, 1.0))
            .insert(Velocity {
                linvel: laser_velocity + firer_velocity,
                ..Default::default()
            });

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
            commands,
        )
    }
}

fn handle_events(
    mut query: Query<(Entity, Option<&NoCollide>, &mut Laser, &CollidingEntities), With<Laser>>,
    mut commands: Commands,
) {
    for (laser_entity, no_collide_entity, mut laser, collided_with_entities) in query.iter_mut() {
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

                laser.active = false;
                println!(
                    "BANG! Hit {}! Time to despawn self!",
                    collided_with_entity.index()
                );
                commands.entity(laser_entity).despawn();
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(handle_events);
}
