use bevy::math::Vec3;
use cosmos_core::physics::location::{Location, Sector, SectorUnit};

use crate::commands::prelude::ArgumentError;

#[derive(Clone, Copy, Debug)]
pub enum CommandLocation {
    Absolute(Location),
    Relative(Vec3),
}

impl Default for CommandLocation {
    fn default() -> Self {
        Self::Relative(Vec3::ZERO)
    }
}

impl CommandLocation {
    pub fn to_location(self, sender_loc: Option<&Location>) -> Option<Location> {
        match self {
            Self::Absolute(a) => Some(a),
            Self::Relative(r) => sender_loc.map(|l| *l + r),
        }
    }
}

pub fn parse_location(args: &[String]) -> Result<(CommandLocation, usize), ArgumentError> {
    if args.len() == 0 {
        return Ok((CommandLocation::Relative(Vec3::ZERO), 0));
        // return Err(ArgumentError::TooFewArguments);
    }

    let mut spawn_at = Location::default();

    let mut n = 0;

    if args.len() >= 3 {
        let x = args[0].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
            arg_index: 0,
            type_name: "SectorUnit".into(),
        })?;
        let y = args[1].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
            arg_index: 1,
            type_name: "SectorUnit".into(),
        })?;
        let z = args[2].parse::<SectorUnit>().map_err(|_| ArgumentError::InvalidType {
            arg_index: 2,
            type_name: "SectorUnit".into(),
        })?;

        spawn_at.sector = Sector::new(x, y, z);
        n = 3;

        if args.len() >= 6 {
            let x = args[3].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 3,
                type_name: "SectorUnit".into(),
            })?;
            let y = args[4].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 4,
                type_name: "SectorUnit".into(),
            })?;
            let z = args[5].parse::<f32>().map_err(|_| ArgumentError::InvalidType {
                arg_index: 5,
                type_name: "SectorUnit".into(),
            })?;
            spawn_at.local = Vec3::new(x, y, z);
            n = 6;
        } else if args.len() != 3 {
            return Err(ArgumentError::TooFewArguments);
        }
    } else if args.len() != 2 {
        return Err(ArgumentError::TooFewArguments);
    }

    Ok((CommandLocation::Absolute(spawn_at), n))
}
