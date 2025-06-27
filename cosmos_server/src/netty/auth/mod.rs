use bevy::prelude::*;
// use steamworks::ServerMode;
//
// #[derive(Resource)]
// pub enum AuthenticationServer {
//     Steam(steamworks::Server),
//     None,
// }
//
// fn create_steam_server(mut commands: Commands) {
//     let port: u16 = 1337;
//
//     info!("Creating steam server...");
//
//     let (steam_server, _) =
//         steamworks::Server::init("0.0.0.0".parse().unwrap(), port, port + 1, ServerMode::Authentication, "0.0.9a").unwrap();
//
//     info!("Created steam server!");
//
//     commands.insert_resource(AuthenticationServer::Steam(steam_server));
// }

pub(super) fn register(app: &mut App) {
    // app.add_systems(Startup, create_steam_server);
}
