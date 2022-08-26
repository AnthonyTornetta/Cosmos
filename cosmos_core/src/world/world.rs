use rapier3d::na::Vector3;
use crate::structure::structure::Structure;
use rapier3d::prelude::*;

struct PhysicsInformation {
    gravity: Vector3<f32>,
    physics_pipeline: PhysicsPipeline,
    integration_parameters: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    physics_hooks: (),
    event_handler: (),

    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet
}

struct ChunkPhysics {

}

pub struct World {
    structures: Vec<Structure>,
    physics_info: PhysicsInformation
}

impl World {
    pub fn new() -> Self {
        Self {
            structures: Vec::new(),
            physics_info: PhysicsInformation {
                gravity: vector![0.0, 0.0, 0.0],
                integration_parameters: IntegrationParameters::default(),
                physics_pipeline: PhysicsPipeline::new(),
                island_manager: IslandManager::new(),
                broad_phase: BroadPhase::new(),
                narrow_phase: NarrowPhase::new(),
                impulse_joint_set: ImpulseJointSet::new(),
                multibody_joint_set: MultibodyJointSet::new(),
                ccd_solver: CCDSolver::new(),
                physics_hooks: (),
                event_handler: (),
                rigid_body_set: RigidBodySet::new(),
                collider_set: ColliderSet::new()
            }
        }
    }

    pub fn add_structure(&mut self, structure: Structure) {

        ColliderBuilder::cuboid(16.0, 16.0, 16.0);

        let handle = self.physics_info.rigid_body_set.insert(structure.body);

        self.physics_info.collider_set.insert_with_parent(collider, )

        self.structures.push(structure);
    }

    pub fn update(&mut self) {
        let mut info = &mut self.physics_info;

        info.physics_pipeline.step(
            &info.gravity,
            &info.integration_parameters,
            &mut info.island_manager,
            &mut info.broad_phase,
            &mut info.narrow_phase,
            &mut info.rigid_body_set,
            &mut info.collider_set,
            &mut info.impulse_joint_set,
            &mut info.multibody_joint_set,
            &mut info.ccd_solver,
            &mut info.physics_hooks,
            &mut info.event_handler
        );
    }
}