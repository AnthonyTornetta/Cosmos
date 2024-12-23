//! Handles shared logic for the gravity well

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        system::{Query, Res},
    },
    math::{Quat, Vec3},
    prelude::IntoSystemConfigs,
    reflect::Reflect,
    time::Time,
    transform::components::GlobalTransform,
};
use bevy_rapier3d::dynamics::{ExternalImpulse, ReadMassProperties};
use serde::{Deserialize, Serialize};

use crate::{
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    structure::coordinates::BlockCoordinate,
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

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, do_gravity_well.in_set(NetworkingSystemsSet::Between))
        .register_type::<GravityWell>();
}
