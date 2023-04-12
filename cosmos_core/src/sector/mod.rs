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
