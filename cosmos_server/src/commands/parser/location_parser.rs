use bevy::math::Vec3;
use cosmos_core::physics::location::{Location, Sector, SectorUnit};

use crate::commands::prelude::ArgumentError;

#[derive(Clone, Copy, Debug, Default)]
pub struct CommandLocation {
    sector: CommandSectorCoordinate,
    local: CommandCoordinate,
}

impl CommandLocation {
    pub fn to_location(self, sender_loc: Option<&Location>) -> Option<Location> {
        let sector = self.sector.to_location(sender_loc.map(|x| x.sector))?;
        let local = self.local.to_location(sender_loc.map(|x| x.local))?;
        Some(Location::new(local, sector))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CommandSectorUnit {
    Relative(SectorUnit),
    Absolute(SectorUnit),
}

impl Default for CommandSectorUnit {
    fn default() -> Self {
        Self::Relative(0)
    }
}

impl CommandSectorUnit {
    pub fn to_location(self, base_unit: Option<SectorUnit>) -> Option<SectorUnit> {
        match self {
            Self::Absolute(a) => Some(a),
            Self::Relative(r) => base_unit.map(|u| u + r),
        }
    }

    fn set(&mut self, val: SectorUnit) {
        match self {
            Self::Absolute(a) => *a = val,
            Self::Relative(r) => *r = val,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CommandSectorCoordinate {
    pub x: CommandSectorUnit,
    pub y: CommandSectorUnit,
    pub z: CommandSectorUnit,
}

impl CommandSectorCoordinate {
    pub fn to_location(self, base_unit: Option<Sector>) -> Option<Sector> {
        let x = self.x.to_location(base_unit.map(|v| v.x()))?;
        let y = self.y.to_location(base_unit.map(|v| v.y()))?;
        let z = self.z.to_location(base_unit.map(|v| v.z()))?;
        Some(Sector::new(x, y, z))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CommandLocalUnit {
    Relative(f32),
    Absolute(f32),
}

impl Default for CommandLocalUnit {
    fn default() -> Self {
        Self::Relative(0.0)
    }
}

impl CommandLocalUnit {
    pub fn to_location(self, base_unit: Option<f32>) -> Option<f32> {
        match self {
            Self::Absolute(a) => Some(a),
            Self::Relative(r) => base_unit.map(|u| u + r),
        }
    }

    fn set(&mut self, val: f32) {
        match self {
            Self::Absolute(a) => *a = val,
            Self::Relative(r) => *r = val,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CommandCoordinate {
    pub x: CommandLocalUnit,
    pub y: CommandLocalUnit,
    pub z: CommandLocalUnit,
}

impl CommandCoordinate {
    pub fn to_location(self, base_unit: Option<Vec3>) -> Option<Vec3> {
        let x = self.x.to_location(base_unit.map(|v| v.x))?;
        let y = self.y.to_location(base_unit.map(|v| v.y))?;
        let z = self.z.to_location(base_unit.map(|v| v.z))?;
        Some(Vec3::new(x, y, z))
    }
}

fn parse_sector(sector_unit: &mut CommandSectorUnit, args: &[String], idx: usize) -> Result<(), ArgumentError> {
    let mut arg = args[idx].as_str();
    if arg.starts_with("~") {
        *sector_unit = CommandSectorUnit::Relative(0);
        arg = &arg[1..];
        if arg.is_empty() {
            return Ok(());
        }
    } else {
        *sector_unit = CommandSectorUnit::Absolute(0);
    }
    let coord = arg.parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
        arg_index: idx as u32,
        type_name: "SectorUnit".into(),
    })?;
    sector_unit.set(coord);
    Ok(())
}

fn parse_local(coordinate_unit: &mut CommandLocalUnit, args: &[String], idx: usize) -> Result<(), ArgumentError> {
    let mut arg = args[idx].as_str();
    if arg.starts_with("~") {
        *coordinate_unit = CommandLocalUnit::Relative(0.0);
        arg = &arg[1..];
        if arg.is_empty() {
            return Ok(());
        }
    } else {
        *coordinate_unit = CommandLocalUnit::Absolute(0.0);
    }
    let coord = arg.parse::<f32>().map_err(|_| ArgumentError::InvalidType {
        arg_index: idx as u32,
        type_name: "f32".into(),
    })?;
    coordinate_unit.set(coord);
    Ok(())
}

pub fn parse_location(args: &[String]) -> Result<(CommandLocation, usize), ArgumentError> {
    if args.len() == 0 {
        return Ok((CommandLocation::default(), 0));
    }

    let mut spawn_at = CommandLocation::default();

    let mut n = 0;

    if args.len() >= 3 {
        parse_sector(&mut spawn_at.sector.x, args, 0)?;
        parse_sector(&mut spawn_at.sector.y, args, 1)?;
        parse_sector(&mut spawn_at.sector.z, args, 2)?;

        n = 3;

        if args.len() >= 6 {
            parse_local(&mut spawn_at.local.x, args, 3)?;
            parse_local(&mut spawn_at.local.y, args, 4)?;
            parse_local(&mut spawn_at.local.z, args, 5)?;

            n = 6;
        } else if args.len() != 3 {
            return Err(ArgumentError::TooFewArguments);
        }
    } else if args.len() != 2 {
        return Err(ArgumentError::TooFewArguments);
    }

    Ok((spawn_at, n))
}
