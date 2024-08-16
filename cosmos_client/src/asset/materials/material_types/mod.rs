use bevy::prelude::App;

pub mod animated_material;
pub mod lod_material;
pub mod main_material;

pub(super) fn register(app: &mut App) {
    lod_material::register(app);
    main_material::register(app);
    animated_material::register(app);
}
