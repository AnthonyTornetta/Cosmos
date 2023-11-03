use bevy::prelude::App;

pub mod main_material;

pub(super) fn register(app: &mut App) {
    main_material::register(app);
}
