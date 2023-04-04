use bevy::{
    prelude::{
        App, Commands, Entity, IntoSystemConfigs, Query, Res, ResMut, Resource, Transform, With,
    },
    utils::HashSet,
};
use bevy_rapier3d::prelude::RigidBodyDisabled;

use crate::{
    physics::location::Location,
    structure::{loading::ChunksNeedLoaded, ship::Ship},
};

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
            println!("Saving @ {}", loc);
            commands.entity(ent).insert(RigidBodyDisabled);
            // .insert(OgRb(*rb))
            // .insert(RigidBody::Fixed);
            println!("Locking!");
        } else if loading_sectors.removed.contains(&coords) {
            // if let Some(og_rb) = og_rb {
            commands.entity(ent).remove::<RigidBodyDisabled>();

            println!("Unlocking!");
            // }
        }
    }
}

fn print_ship_loc(query: Query<(&Location, &Transform), With<Ship>>) {
    for (x, y) in query.iter() {
        println!("{x} {:.1}", y.translation);
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(LoadingSectors::default())
        .add_systems((monitor_sectors, freeze_sectors).chain())
        .add_system(print_ship_loc);
}
