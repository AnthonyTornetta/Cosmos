//! Represents the shield functionality

use bevy::{
    app::App,
    ecs::{component::Component, system::Resource},
    reflect::Reflect,
    utils::hashbrown::HashMap,
};
use bigdecimal::num_traits::Pow;
use serde::{Deserialize, Serialize};

use crate::structure::coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate};

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShieldProjectorProperty {
    pub shield_strength: f32,
}

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShieldGeneratorProperty {
    /// 1.0 = every J of power turns into 1 Unit if shielding
    pub peak_efficiency: f32,
    pub power_usage_per_sec: f32,
}

#[derive(Resource, Default)]
pub struct ShieldProjectorBlocks(pub HashMap<u16, ShieldProjectorProperty>);

#[derive(Resource, Default)]
pub struct ShieldGeneratorBlocks(pub HashMap<u16, ShieldGeneratorProperty>);

#[derive(Reflect, Default, Component, Clone, Serialize, Deserialize, Debug)]
pub struct ShieldSystem {
    projectors: HashMap<BlockCoordinate, ShieldProjectorProperty>,
    generators: HashMap<BlockCoordinate, ShieldGeneratorProperty>,

    needs_shields_recalculated: bool,
    shields: Vec<(BlockCoordinate, ShieldDetails)>,
}

#[derive(Reflect, Default, Component, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct ShieldDetails {
    pub max_strength: f32,
    /// shield units/seconds
    pub generation_power_per_sec: f32,
    /// Power per Shield Unit (p/su) -- 0.5 means 2 power per shield unit
    pub generation_efficiency: f32,
    pub radius: f32,
}

impl ShieldSystem {
    pub fn needs_shields_recalculated(&self) -> bool {
        self.needs_shields_recalculated
    }

    pub fn projector_removed(&mut self, coords: BlockCoordinate) {
        self.projectors.remove(&coords);
        self.needs_shields_recalculated = true;
    }

    pub fn projector_added(&mut self, property: ShieldProjectorProperty, coords: BlockCoordinate) {
        self.projectors.insert(coords, property);
        self.needs_shields_recalculated = true;
    }

    pub fn generator_removed(&mut self, coords: BlockCoordinate) {
        self.generators.remove(&coords);
        self.needs_shields_recalculated = true;
    }

    pub fn generator_added(&mut self, property: ShieldGeneratorProperty, coords: BlockCoordinate) {
        self.generators.insert(coords, property);
        self.needs_shields_recalculated = true;
    }

    pub fn shield_details(&self) -> &[(BlockCoordinate, ShieldDetails)] {
        &self.shields
    }

    const DIRS: [UnboundBlockCoordinate; 6] = [
        UnboundBlockCoordinate::new(0, -1, 0),
        UnboundBlockCoordinate::new(0, 1, 0),
        UnboundBlockCoordinate::new(-1, 0, 0),
        UnboundBlockCoordinate::new(1, 0, 0),
        UnboundBlockCoordinate::new(0, 0, -1),
        UnboundBlockCoordinate::new(0, 0, 1),
    ];

    pub fn recalculate_shields(&mut self) {
        self.needs_shields_recalculated = false;

        let mut projectors = self.projectors.clone();

        self.shields.clear();

        // Group projectors into their respective groups
        let mut projector_coord_groups = vec![];

        while !projectors.is_empty() {
            let (&bc, _) = projectors.iter().next().expect("This is not empty.");
            projectors.remove(&bc).expect("Guarenteed");

            let mut min_bounds = bc;
            let mut max_bounds = bc;

            let mut doing = vec![bc];

            let mut projector_coords = vec![];

            while !doing.is_empty() {
                let mut new_doing = vec![];

                for coord in doing {
                    projector_coords.push(coord);

                    if coord.x < min_bounds.x {
                        min_bounds.x = coord.x;
                    }
                    if coord.y < min_bounds.y {
                        min_bounds.y = coord.y;
                    }
                    if coord.z < min_bounds.z {
                        min_bounds.z = coord.z;
                    }

                    if coord.x > max_bounds.x {
                        max_bounds.x = coord.x;
                    }
                    if coord.y > max_bounds.y {
                        max_bounds.y = coord.y;
                    }
                    if coord.z > max_bounds.z {
                        max_bounds.z = coord.z;
                    }

                    if let Ok(bc) = coord.left() {
                        if projectors.remove(&bc).is_some() {
                            new_doing.push(bc);
                        }
                    }
                    if let Ok(bc) = coord.bottom() {
                        if projectors.remove(&bc).is_some() {
                            new_doing.push(bc);
                        }
                    }
                    if let Ok(bc) = coord.back() {
                        if projectors.remove(&bc).is_some() {
                            new_doing.push(bc);
                        }
                    }
                    let bc = coord.top();
                    if projectors.remove(&bc).is_some() {
                        new_doing.push(bc);
                    }
                    let bc = coord.right();
                    if projectors.remove(&bc).is_some() {
                        new_doing.push(bc);
                    }
                    let bc = coord.front();
                    if projectors.remove(&bc).is_some() {
                        new_doing.push(bc);
                    }
                }

                doing = new_doing;
            }

            projector_coord_groups.push((projector_coords, min_bounds, max_bounds));
        }

        // Step 2: Go through each group & calculate its properties

        for (group, min_bounds, max_bounds) in projector_coord_groups {
            let mut group_generators = HashMap::<BlockCoordinate, u8>::default();

            let mut shield_details = ShieldDetails::default();

            let mut center = BlockCoordinate::splat(0);

            let group_len = group.len();

            for projector_coord in group {
                center = center + projector_coord;

                // calculate projector effectiveness
                let mut touching_projectors = 0;

                if let Ok(bc) = projector_coord.left() {
                    if self.projectors.contains_key(&bc) {
                        touching_projectors += 1;
                    } else if self.generators.contains_key(&bc) {
                        let value = *group_generators.entry(bc).or_default() + 1;
                        group_generators.insert(bc, value);
                    }
                }
                if let Ok(bc) = projector_coord.bottom() {
                    if self.projectors.contains_key(&bc) {
                        touching_projectors += 1;
                    } else if self.generators.contains_key(&bc) {
                        let value = *group_generators.entry(bc).or_default() + 1;
                        group_generators.insert(bc, value);
                    }
                }
                if let Ok(bc) = projector_coord.back() {
                    if self.projectors.contains_key(&bc) {
                        touching_projectors += 1;
                    } else if self.generators.contains_key(&bc) {
                        let value = *group_generators.entry(bc).or_default() + 1;
                        group_generators.insert(bc, value);
                    }
                }
                let bc = projector_coord.top();
                if self.projectors.contains_key(&bc) {
                    touching_projectors += 1;
                } else if self.generators.contains_key(&bc) {
                    let value = *group_generators.entry(bc).or_default() + 1;
                    group_generators.insert(bc, value);
                }
                let bc = projector_coord.right();
                if self.projectors.contains_key(&bc) {
                    touching_projectors += 1;
                } else if self.generators.contains_key(&bc) {
                    let value = *group_generators.entry(bc).or_default() + 1;
                    group_generators.insert(bc, value);
                }
                let bc = projector_coord.front();
                if self.projectors.contains_key(&bc) {
                    touching_projectors += 1;
                } else if self.generators.contains_key(&bc) {
                    let value = *group_generators.entry(bc).or_default() + 1;
                    group_generators.insert(bc, value);
                }

                let property = self.projectors.get(&projector_coord).expect("This must exist");
                shield_details.max_strength += property.shield_strength * (touching_projectors as f32 + 1.0).pow(1.2);
            }

            let (generator_power_per_sec, generator_efficiency) = group_generators
                .iter()
                .map(|(coord, &total)| {
                    let gen_property = self.generators.get(coord).expect("This must exist");

                    (
                        gen_property.power_usage_per_sec,
                        gen_property.peak_efficiency.pow((7 - total) as f32),
                    )
                })
                .reduce(|(a, b), (x, y)| (a + x, b + y))
                .map(|(power_per_sec, efficiency)| (power_per_sec, efficiency / group_generators.len() as f32))
                .unwrap_or((0.0, 0.0));

            shield_details.generation_efficiency = generator_efficiency;
            shield_details.generation_power_per_sec = generator_power_per_sec;

            center.x /= group_len as CoordinateType;
            center.y /= group_len as CoordinateType;
            center.z /= group_len as CoordinateType;

            let distance = max_bounds - min_bounds;
            shield_details.radius = distance.x.min(distance.y).min(distance.z) as f32 * 2.0 + 10.0;

            self.shields.push((center, shield_details));
        }
    }
}

impl StructureSystemImpl for ShieldSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:shield"
    }
}

impl SyncableSystem for ShieldSystem {}

pub(super) fn register(app: &mut App) {
    app.register_type::<ShieldSystem>();
}
