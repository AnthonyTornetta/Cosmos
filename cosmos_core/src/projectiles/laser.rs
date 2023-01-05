use bevy::prelude::{
    App, Commands, Component, Entity, PbrBundle, Quat, Query, Transform, Vec3, With,
};
use bevy_rapier3d::prelude::CollidingEntities;

#[derive(Component)]
pub struct NoCollide(Entity);

#[derive(Component)]
pub struct Laser {
    // strength: f32,
    active: bool, // commands despawning entity isn't instant, but changing this field is.
}

/// Spawns a laser with the given position & velocity
/// Base strength is 100
///
pub fn spawn_laser(
    position: Vec3,
    velocity: Vec3,
    _strength: f32,
    no_collide_entity: Option<Entity>,
    commands: &mut Commands,
) {
    let mut transform = Transform {
        translation: position,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    transform.look_at(velocity, Vec3::Y);

    let mut ecmds = commands.spawn(PbrBundle {
        transform,
        ..Default::default()
    });

    ecmds
        .insert(Laser {
            // strength,
            active: true,
        })
        .insert(CollidingEntities::default());

    if let Some(ent) = no_collide_entity {
        ecmds.insert(NoCollide(ent));
    }
}

fn handle_events(
    mut query: Query<(Entity, Option<&NoCollide>, &mut Laser, &CollidingEntities), With<Laser>>,
    mut commands: Commands,
) {
    for (entity, no_col, mut laser, c_es) in query.iter_mut() {
        if laser.active {
            for ent in c_es
                .iter()
                .filter(|x| no_col.is_none() || *x != no_col.unwrap().0)
            {
                if !laser.active {
                    break;
                }

                laser.active = false;
                println!("Hit {}! Time to despawn self!", ent.index());
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn register(app: &mut App) {
    app.add_system(handle_events);
}
