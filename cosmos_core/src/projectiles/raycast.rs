use bevy::prelude::*;
use bevy_rapier3d::{plugin::RapierContext, prelude::QueryFilter};

use crate::prelude::Structure;

fn raycast(
    context: &RapierContext,
    no_collide_entity: Option<Entity>,
    ray_start: Vec3,
    ray_dir: Vec3,
    ray_distance: f32,
    q_parent: Query<&ChildOf>,
    pen: u32,
    power: f32,
    q_structure: Query<&Structure>,
) {
    let filter = QueryFilter::predicate(QueryFilter::default(), &|entity| {
        if let Some(no_collide_entity) = no_collide_entity {
            if no_collide_entity == entity {
                false
            } else if let Ok(parent) = q_parent.get(entity) {
                parent.parent() != no_collide_entity
            } else {
                true
            }
        } else {
            true
        }
    });

    if pen > 1 {
        context.intersections_with_ray(ray_start, ray_dir, ray_distance, false, filter, |hit_ent, intersection| {
            let x = intersection.point;

            let hit_point = ray_start + ray_dir * intersection.time_of_impact;

            intersection.time_of_impact
        });
    } else {
        if let Some((entity, toi)) = context.cast_ray(
            ray_start, // sometimes lasers pass through things that are next to where they are spawned, thus we check starting a bit behind them
            ray_dir,
            ray_distance,
            false,
            filter,
        ) {
            let pos = ray_start + (toi * ray_direction) + (velocity.linvel.normalize() * 0.01);

            if let Ok(parent) = chunk_parent_query.get(entity) {
                if let Ok(transform) = transform_query.get(parent.parent()) {
                    let lph = Quat::from_affine3(&transform.affine())
                        .inverse()
                        .mul_vec3(pos - transform.translation());

                    event_writer.write(LaserCollideMessage {
                        entity_hit: parent.parent(),
                        local_position_hit: lph,
                        laser_strength: laser.strength,
                        causer: causer.copied(),
                    });
                }
            } else if let Ok(transform) = transform_query.get(entity) {
                let lph = Quat::from_affine3(&transform.affine())
                    .inverse()
                    .mul_vec3(pos - transform.translation());

                event_writer.write(LaserCollideMessage {
                    entity_hit: entity,
                    local_position_hit: lph,
                    laser_strength: laser.strength,
                    causer: causer.copied(),
                });
            }

            laser.active = false;
            commands.entity(laser_entity).insert(NeedsDespawned);
        }
    }
}
