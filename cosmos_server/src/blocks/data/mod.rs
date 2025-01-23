use bevy::app::App;

mod instances;
pub mod utils;

pub(super) fn register(app: &mut App) {
    instances::register(app);
}
