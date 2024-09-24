use bevy::prelude::App;

#[cfg(feature = "client")]
/// Contains client logic and utilities for client netty event logic
pub mod client_event;
/// Contains shared logic for netty events.
pub mod netty_event;
#[cfg(feature = "server")]
/// Contains server logic and utilities for server netty event logic
pub mod server_event;

pub(super) fn register(app: &mut App) {
    netty_event::register(app);

    #[cfg(feature = "server")]
    server_event::register(app);
    #[cfg(feature = "client")]
    client_event::register(app);
}
