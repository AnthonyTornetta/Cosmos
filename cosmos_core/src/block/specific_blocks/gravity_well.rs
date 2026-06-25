//! Handles shared logic for the gravity well

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_rapier3d::{
    dynamics::{ExternalImpulse, ReadMassProperties},
    prelude::AdditionalMassProperties,
};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::sets::FixedUpdateSet, netty::sync::IdentifiableComponent, structure::coordinates::BlockCoordinate,
    utils::ecs::register_fixed_update_removed_component,
};

/// This component indicates the entity is under the affects of a gravity well.
#[derive(Serialize, Deserialize, Component, Clone, Copy, Debug, Reflect)]
pub struct GravityWell {
    /// g_constant * mass = force
    pub g_constant: Vec3,
    /// The structure this gravity well is for
    pub structure_entity: Entity,
    /// The block this gravity well is from
    pub block: BlockCoordinate,
}

impl IdentifiableComponent for GravityWell {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:gravity_well"
    }
}

fn do_gravity_well(
    time: Res<Time>,
    mut q_grav_well: Query<(&GravityWell, &ReadMassProperties, &mut ExternalImpulse)>,
    q_global_trans: Query<&GlobalTransform>,
) {
    for (under_gravity_well, read_mass_props, mut ext_impulse) in q_grav_well.iter_mut() {
        let Ok(g_trans) = q_global_trans.get(under_gravity_well.structure_entity) else {
            continue;
        };

        ext_impulse.impulse +=
            Quat::from_affine3(&g_trans.affine()).mul_vec3(under_gravity_well.g_constant * read_mass_props.mass * time.delta_secs());
    }
}

// TODO: This is a hack because the physics engine is kinda buggy. Check back on this in the
// future.
fn update_mass_props(mut commands: Commands, q_ent: Query<Entity, With<ReadMassProperties>>) {
    for e in q_ent.iter() {
        // Recalculates the [`ReadMassProperties`], which can be unreliable
        commands.entity(e).insert(AdditionalMassProperties::Mass(0.0));
    }
}

pub(super) fn register(app: &mut App) {
    register_fixed_update_removed_component::<GravityWell>(app);
    app.add_systems(
        FixedUpdate,
        (update_mass_props.run_if(on_timer(Duration::from_secs(5))), do_gravity_well)
            .chain()
            .in_set(FixedUpdateSet::Main),
    )
    .register_type::<GravityWell>();
}
