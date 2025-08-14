// use bevy::prelude::*;
//
// use crate::netty::sync::events::{client_event::NettyEventWriter, netty_event::NettyEvent};
//
// pub fn fetch<S: NettyEvent, R: NettyEvent>(payload: &S) {
//     move |mut nevw_send: NettyEventWriter<S>, mut commands: Commands| {
//         commands.run_system();
//     }
// }
//
// pub(super) fn register(app: &mut App) {}
