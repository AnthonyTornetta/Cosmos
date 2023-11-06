use bevy::prelude::App;

pub mod animated_material;
pub mod main_material;

pub(super) fn register(app: &mut App) {
    main_material::register(app);
    animated_material::register(app);
}
