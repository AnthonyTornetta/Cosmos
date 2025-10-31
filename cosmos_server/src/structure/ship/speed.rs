use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    physics::location::Location,
    prelude::{Planet, Ship, Structure, StructureLoadingSet},
};

const DEFAULT_MAX_SHIP_SPEED: f32 = 350.0;

#[derive(Debug, Reflect)]
pub struct ShipSpeedModifier {
    max_speed: f32,
    impact: f32,
}

impl ShipSpeedModifier {
    pub fn new(max_speed: f32, impact: f32) -> Self {
        Self { max_speed, impact }
    }
}

#[derive(Component, Debug, Reflect, Default)]
pub struct MaxShipSpeed {
    modifiers: HashMap<&'static str, ShipSpeedModifier>,
}

impl MaxShipSpeed {
    pub fn new(name: &'static str, modifier: ShipSpeedModifier) -> Self {
        let mut this = Self {
            modifiers: HashMap::default(),
        };

        this.add_modifier(name, modifier);

        this
    }

    pub fn max_speed(&self) -> f32 {
        let total_percent = self.modifiers.iter().map(|(_, x)| x.impact).sum::<f32>();

        let mut max_speed = 0.0;

        if total_percent < 1.0 {
            max_speed += DEFAULT_MAX_SHIP_SPEED * (1.0 - total_percent);
        }

        if total_percent != 0.0 {
            max_speed += self
                .modifiers
                .iter()
                .map(|(_, x)| x.max_speed * x.impact / total_percent)
                .sum::<f32>();
        }

        max_speed
    }

    pub fn add_modifier(&mut self, name: &'static str, modifier: ShipSpeedModifier) {
        self.modifiers.insert(name, modifier);
    }

    pub fn remove_modifier(&mut self, name: &'static str) {
        self.modifiers.remove(&name);
    }
}

const REASON: &str = "cosmos:planet";

const MAX_PLANET_SPEED: f32 = 50.0;

fn add_planet_modifier(
    mut q_ship: Query<(&Location, &mut MaxShipSpeed), With<Ship>>,
    q_planet: Query<(&Location, &Structure, &GlobalTransform), With<Planet>>,
) {
    for (ship_loc, mut max_speed) in q_ship.iter_mut() {
        let Some((planet_loc, planet_structure, g_trans)) = q_planet
            .iter()
            .filter(|(l, _, _)| l.is_within_reasonable_range(ship_loc))
            .min_by_key(|(l, _, _)| l.distance_sqrd(ship_loc) as i32)
        else {
            max_speed.remove_modifier(REASON);
            continue;
        };

        let delta = (g_trans.rotation().inverse() * (*ship_loc - *planet_loc).absolute_coords_f32()).abs();
        let square_dist = delta.x.max(delta.y).max(delta.z);

        // All sides are the same side
        let square_radius = planet_structure.block_dimensions().x as f32 / 2.0;

        info!("Delta: {delta} vs {square_radius}");

        let impact = (square_radius.powf(2.0) / square_dist.powf(2.0)).clamp(0.0, 1.0);
        info!("Impact: {impact}");
        if impact < 0.1 {
            max_speed.remove_modifier(REASON);
        } else {
            max_speed.add_modifier(REASON, ShipSpeedModifier::new(MAX_PLANET_SPEED, impact));
        }
    }
}

fn add_max_speed(mut commands: Commands, q_ship: Query<Entity, (With<Ship>, Without<MaxShipSpeed>)>) {
    for ent in q_ship.iter() {
        commands.entity(ent).insert(MaxShipSpeed::default());
    }
}

fn limit_speed(mut q_ship: Query<(&mut Velocity, &MaxShipSpeed), (With<Ship>, Or<(Changed<Velocity>, Changed<MaxShipSpeed>)>)>) {
    for (mut vel, max_speed) in q_ship.iter_mut() {
        let max_speed = max_speed.max_speed();

        vel.linvel = vel.linvel.clamp_length(0.0, max_speed);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            add_max_speed.in_set(StructureLoadingSet::StructureLoaded),
            limit_speed.in_set(FixedUpdateSet::PrePhysics),
        ),
    )
    .add_systems(
        FixedUpdate,
        add_planet_modifier.in_set(FixedUpdateSet::PostLocationSyncingPostPhysics),
    );
}
