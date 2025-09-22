use bevy::app::App;

mod reactor;
mod shipyard;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
    shipyard::register(app);
}
