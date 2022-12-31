use bevy::prelude::App;

pub mod sync;

pub fn register(app: &mut App) {
    println!("REGISTERING INVENTORY!!!");
    sync::register(app);
}
