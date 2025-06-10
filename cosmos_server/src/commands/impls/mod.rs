use bevy::prelude::*;

mod blueprint;
mod blueprints;
mod despawn;
mod give;
mod items;
mod list;
mod load;
mod ping;
mod say;

pub(super) fn register(app: &mut App) {
    ping::register(app);
    blueprint::register(app);
    load::register(app);
    say::register(app);
    list::register(app);
    despawn::register(app);
    blueprints::register(app);
    give::register(app);
    items::register(app);
}
