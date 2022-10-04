use bevy::prelude::App;

pub mod ship;

pub fn regsiter(app: &mut App) {
    ship::register(app);
}
