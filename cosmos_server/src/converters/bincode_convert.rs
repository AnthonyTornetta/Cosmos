use std::ffi::OsStr;

use bevy::prelude::*;
use cosmos_core::{
    physics::location::{Location, systems::Anchor},
    state::GameState,
};
use walkdir::WalkDir;

use crate::persistence::{loading::NeedsBlueprintLoaded, saving::NeedsBlueprinted};

use super::SaveVersion;

fn load_all_blueprints(mut commands: Commands, mut ran: Local<bool>) {
    if *ran {
        return;
    }
    *ran = true;

    commands.spawn((Anchor, Transform::default(), Location::default()));

    let types = ["ship", "station"];

    for t in types {
        for bp in WalkDir::new(format!("blueprints/{t}")).max_depth(1).into_iter().flatten() {
            if !(bp.file_type().is_file() && bp.path().extension() == Some(OsStr::new("bp"))) {
                continue;
            }

            info!("doing {bp:?}");

            let mut blueprint_name = bp.file_name().to_str().unwrap().to_owned();
            blueprint_name = blueprint_name[0..(blueprint_name.len() - ".bp".len())].to_owned();

            commands.spawn((
                NeedsBlueprintLoaded {
                    path: bp.path().to_str().unwrap().to_owned(),
                    spawn_at: Default::default(),
                    rotation: Default::default(),
                },
                NeedsBlueprinted {
                    subdir_name: t.to_owned(),
                    blueprint_name,
                },
            ));
        }
    }
}

pub(super) fn register(app: &mut App, version: SaveVersion) {
    if version == SaveVersion::Old {
        app.add_systems(FixedUpdate, load_all_blueprints.run_if(in_state(GameState::Playing)));
    }
}
