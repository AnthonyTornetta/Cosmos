//! Represents the shield functionality

use bevy::{platform::collections::HashMap, prelude::*};
use bigdecimal::num_traits::Pow;
use serde::{Deserialize, Serialize};

use crate::{
    ecs::name,
    structure::coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate},
};

use super::{StructureSystemImpl, sync::SyncableSystem};

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// How much strength this shield projector will add to the shield
///
/// This is scaled with the number of projectors touching it exponentially.
/// If this is touching no other projectors, then the strength will be this amount.
pub struct ShieldProjectorProperty {
    /// How much strength this shield projector will add to the shield
    ///
    /// This is scaled with the number of projectors touching it exponentially.
    /// If this is touching no other projectors, then the strength will be this amount.
    pub shield_strength: f32,
}

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
/// This block can be used to generate shield strength
pub struct ShieldGeneratorProperty {
    /// The amount of shield produced per second = `efficiency` * `power_usage_per_sec`
    ///
    /// This peak efficiency is reached when the generator is adjacent to 6 projectors.
    /// Efficiencies lower than 0.70 tend to be very ineffective with few adjacent projectors
    /// because the efficiency calculation is exponential.
    pub peak_efficiency: f32,
    /// How much power this generator can consume per second.
    ///
    /// The amount of shield produced per second = efficiency * this
    pub power_usage_per_sec: f32,
}

#[derive(Resource, Default)]
/// All blocks that can be used to project shields
pub struct ShieldProjectorBlocks(pub HashMap<u16, ShieldProjectorProperty>);

#[derive(Resource, Default)]
/// All blocks that can be used to generate shield strength
pub struct ShieldGeneratorBlocks(pub HashMap<u16, ShieldGeneratorProperty>);

#[derive(Reflect, Default, Component, Clone, Serialize, Deserialize, Debug)]
/// Contains logic for shields
pub struct ShieldSystem {
    projectors: HashMap<BlockCoordinate, ShieldProjectorProperty>,
    generators: HashMap<BlockCoordinate, ShieldGeneratorProperty>,

    needs_shields_recalculated: bool,
    shields: Vec<(BlockCoordinate, ShieldDetails)>,
}

#[derive(Reflect, Default, Component, Clone, Copy, Serialize, Deserialize, Debug)]
/// Contains information about how a shield should be
pub struct ShieldDetails {
    /// The maximum amount of strength this shield can hold
    pub max_strength: f32,
    /// How much power this shield can turn into shield units per second
    pub generation_power_per_sec: f32,
    /// Power per Shield Unit (p/su) -- 0.5 means 2 power per shield unit
    pub generation_efficiency: f32,
    /// The radius of the shield (in meters)
    pub radius: f32,
}

impl ShieldSystem {
    /// Returns true if the [`Self::recalculate_shields`] needs to be called for updated shield information.
    ///
    /// This is useful, because Changed<ShieldSystem> doesn't always mean the shields need to be recalculated.
    pub fn needs_shields_recalculated(&self) -> bool {
        self.needs_shields_recalculated
    }

    /// Call this whenever a projector block is removed from the structure
    pub fn projector_removed(&mut self, coords: BlockCoordinate) {
        self.projectors.remove(&coords);
        self.needs_shields_recalculated = true;
    }

    /// Call this whenever a projector block is added to the structure
    pub fn projector_added(&mut self, property: ShieldProjectorProperty, coords: BlockCoordinate) {
        self.projectors.insert(coords, property);
        self.needs_shields_recalculated = true;
    }

    /// Call this whenever a generator block is removed from the structure
    pub fn generator_removed(&mut self, coords: BlockCoordinate) {
        self.generators.remove(&coords);
        self.needs_shields_recalculated = true;
    }

    /// Call this whenever a generator block is added to the structure
    pub fn generator_added(&mut self, property: ShieldGeneratorProperty, coords: BlockCoordinate) {
        self.generators.insert(coords, property);
        self.needs_shields_recalculated = true;
    }

    /// Returns the shield details generated from [`Self::recalculate_shields`]
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

    /// Calculates all the shields that should be present in this structure.
    ///
    /// This is rather expensive, so only call this if [`Self::needs_shields_recalculated`] is true.
    /// To get the result of this, call [`Self::shield_details`].
    pub fn recalculate_shields(&mut self) {
        self.needs_shields_recalculated = false;

        let mut projectors = self.projectors.clone();
        let mut generators = self.generators.clone();

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

                    for dir in Self::DIRS {
                        if let Ok(bc) = BlockCoordinate::try_from(dir + coord)
                            && (projectors.remove(&bc).is_some() || generators.remove(&bc).is_some())
                        {
                            new_doing.push(bc);
                        }
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

            let mut n_projectors = 0;

            for projector_coord in group {
                // These can be generators, which are calculated below.
                let Some(property) = self.projectors.get(&projector_coord) else {
                    continue;
                };

                n_projectors += 1;
                center = center + projector_coord;

                // calculate projector effectiveness
                let mut touching_projectors = 0;

                for dir in Self::DIRS {
                    if let Ok(bc) = BlockCoordinate::try_from(dir + projector_coord) {
                        if self.projectors.contains_key(&bc) {
                            touching_projectors += 1;
                        } else if self.generators.contains_key(&bc) {
                            let value = *group_generators.entry(bc).or_default() + 1;
                            group_generators.insert(bc, value);
                        }
                    }
                }

                shield_details.max_strength += property.shield_strength * (touching_projectors as f32 + 1.0).pow(1.2);
            }

            if n_projectors == 0 {
                continue;
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

            center.x /= n_projectors as CoordinateType;
            center.y /= n_projectors as CoordinateType;
            center.z /= n_projectors as CoordinateType;

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
    app.register_type::<ShieldSystem>()
        .add_systems(Update, name::<ShieldSystem>("Shield System"));
}
