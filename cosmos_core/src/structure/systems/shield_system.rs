//! Represents the shield functionality

use bevy::{
    app::App,
    ecs::{component::Component, system::Resource},
    reflect::Reflect,
    utils::HashMap,
};
use serde::{Deserialize, Serialize};

use crate::structure::{
    coordinates::{BlockCoordinate, UnboundBlockCoordinate},
    shields::Shield,
};

use super::{sync::SyncableSystem, StructureSystemImpl};

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShieldProjectorProperty {
    pub shield_strength: f32,
    pub shield_range_increase: f32,
}

#[derive(Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ShieldEnergyConverterProperty {
    /// 1.0 = every J of power turns into 1 Unit if shielding
    pub efficiency: f32,
    pub max_power_usage_per_sec: f32,
}

#[derive(Resource, Default)]
pub struct ShieldProjectorBlocks(pub HashMap<u16, ShieldProjectorProperty>);

#[derive(Resource, Default)]
pub struct ShieldEnergyConverterBlocks(pub HashMap<u16, ShieldProjectorProperty>);

#[derive(Reflect, Default, Component, Clone, Serialize, Deserialize, Debug)]
pub struct ShieldSystem {
    projectors: HashMap<BlockCoordinate, ShieldProjectorProperty>,
    converters: HashMap<BlockCoordinate, ShieldEnergyConverterProperty>,

    needs_shields_recalculated: bool,
    shields: Vec<(BlockCoordinate, ShieldDetails)>,
}

#[derive(Reflect, Default, Component, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct ShieldDetails {
    pub max_strength: f32,
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

    pub fn converter_removed(&mut self, coords: BlockCoordinate) {
        self.converters.remove(&coords);
        self.needs_shields_recalculated = true;
    }

    pub fn converter_added(&mut self, property: ShieldEnergyConverterProperty, coords: BlockCoordinate) {
        self.converters.insert(coords, property);
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

        // 1. find shield centers
        let mut centers = vec![];

        for (&coord, _) in self.projectors.iter() {
            let mut neighbors = 0;
            if coord.left().map(|x| self.projectors.contains_key(&x)).unwrap_or(false) {
                neighbors += 1;
            }
            if coord.bottom().map(|x| self.projectors.contains_key(&x)).unwrap_or(false) {
                neighbors += 1;
            }
            if coord.back().map(|x| self.projectors.contains_key(&x)).unwrap_or(false) {
                neighbors += 1;
            }
            if self.projectors.contains_key(&coord.right()) {
                neighbors += 1;
            }
            if self.projectors.contains_key(&coord.top()) {
                neighbors += 1;
            }
            if self.projectors.contains_key(&coord.front()) {
                neighbors += 1;
            }

            if neighbors == 6 {
                centers.push(coord);
            }
        }

        // 2. calculate min length (that is shield radius)

        self.shields.clear();

        for &center in centers.iter() {
            let mut min_radius = usize::MAX;

            for dir in Self::DIRS {
                min_radius = min_radius.min(self.count_projector_length(center, dir, min_radius));
            }

            let shield = ShieldDetails {
                max_strength: min_radius as f32 * 100.0 * 12.0,
                radius: min_radius as f32 * 6.0 + 10.0,
            };

            self.shields.push((center, shield));
        }

        // 3. calculate generation amount
    }

    fn count_projector_length(&mut self, center: BlockCoordinate, dir: UnboundBlockCoordinate, min_radius: usize) -> usize {
        let mut at = center + dir;

        let mut len = 0;
        while BlockCoordinate::try_from(at)
            .map(|x| self.projectors.get(&x).is_some())
            .unwrap_or(false)
        {
            // Ensure the only projectors touching this projector are in the direction we are travelling/coming from
            if Self::DIRS.into_iter().filter(|&d| d != dir && d != -dir).any(|d| {
                BlockCoordinate::try_from(at + d)
                    .map(|x| self.projectors.get(&x).is_some())
                    .unwrap_or(true)
            }) {
                break;
            }

            len += 1;
            if len > min_radius {
                break;
            }
            at = at + dir;
        }
        len
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
