use bevy::prelude::*;

mod blueprint;
mod blueprints;
mod despawn;
mod gamemode;
mod give;
mod items;
mod list;
mod load;
mod op;
mod ping;
mod say;
mod stop;

pub(super) fn register(app: &mut App) {
    ping::register(app);
    blueprint::register(app);
    load::register(app);
    say::register(app);
    list::register(app);
    despawn::register(app);
    gamemode::register(app);
    blueprints::register(app);
    give::register(app);
    items::register(app);
    op::register(app);
    stop::register(app);
}
