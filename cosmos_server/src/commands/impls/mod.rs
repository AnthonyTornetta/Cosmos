use bevy::prelude::*;
use cosmos_core::state::GameState;

mod ban;
mod blueprint;
mod blueprints;
mod despawn;
mod gamemode;
mod give;
mod items;
mod kick;
mod kill;
mod list;
mod load;
mod op;
mod panic;
mod ping;
mod save;
mod say;
mod spawn;
mod stop;
mod tp;
mod unban;
mod whitelist;

fn display_basic_info() {
    info!("Server fully initialized. Listening for connections...");
    info!(
        "Type `stop` to stop the server gracefully. Do NOT exit the process any other way - you may corrupt your save. Type `help` to view a full list of commands."
    );
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), display_basic_info);

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
    save::register(app);
    spawn::register(app);
    tp::register(app);
    kill::register(app);
    panic::register(app);
    whitelist::register(app);
    ban::register(app);
    kick::register(app);
    unban::register(app);
}
