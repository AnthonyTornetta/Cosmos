//! This whole module is pretty stupid, but it works "well enough" for now. This freezes
//! sectors that have loading structures in them to prevent ships from flying through
//! planets or other ships while they are still loading.
//!
//! It does this by freezing all rigid bodies in loading sectors + enabling them once every
//! structure has loaded. Again, this isn't great and will have to be redone once cube planets
//! are implemented since they will never be fully loaded.
//!
//! Perhaps in the future, only freeze something if it is within X units of an unloaded chunk?

use bevy::{
    prelude::{App, Commands, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, With},
    utils::HashSet,
};
use bevy_rapier3d::prelude::RigidBodyDisabled;

use crate::{physics::location::Location, structure::loading::ChunksNeedLoaded};

type SectorLoc = (i64, i64, i64);

#[derive(Resource, Default, Debug)]
struct LoadingSectors {
    sectors: HashSet<SectorLoc>,
    removed: HashSet<SectorLoc>,
    added: HashSet<SectorLoc>,
}

/// Looks for any sector with a loading structure
fn monitor_sectors(
    query: Query<&Location, With<ChunksNeedLoaded>>,
    mut loading_sectors: ResMut<LoadingSectors>,
) {
    let mut sectors = HashSet::new();
    let mut added = HashSet::new();

    for loc in query.iter() {
        let coords = (loc.sector_x, loc.sector_y, loc.sector_z);

        if !loading_sectors.sectors.contains(&coords) {
            added.insert(coords);
        }

        sectors.insert(coords);
    }

    let removed: HashSet<SectorLoc> = loading_sectors
        .sectors
        .iter()
        .filter(|x| !sectors.contains(x))
        .copied()
        .collect();

    loading_sectors.sectors = sectors;
    loading_sectors.removed = removed;
    loading_sectors.added = added;
}

/// Freeze all rigid bodies in loading sectors
fn freeze_sectors(
    loading_sectors: Res<LoadingSectors>,

    mut query: Query<(Entity, &Location)>,
    mut commands: Commands,
) {
    for (ent, loc) in query.iter_mut() {
        let coords = (loc.sector_x, loc.sector_y, loc.sector_z);

        if loading_sectors.added.contains(&coords) {
            commands.entity(ent).insert(RigidBodyDisabled);
        } else if loading_sectors.removed.contains(&coords) {
            commands.entity(ent).remove::<RigidBodyDisabled>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(LoadingSectors::default())
        .add_systems((monitor_sectors, freeze_sectors).chain());
}
