use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::{chat::ServerSendChatMessageEvent, netty::sync::events::server_event::NettyEventWriter};

struct SayCommand(String);

impl CosmosCommandType for SayCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        Ok(SayCommand(ev.args.join(" ")))
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SayCommand, _>(
        ServerCommand::new("cosmos:say", "[...message]", "Sends the given text to all connected players"),
        app,
        |mut nevw_send_chat_msg: NettyEventWriter<ServerSendChatMessageEvent>, mut evr_command: EventReader<CommandEvent<SayCommand>>| {
            for ev in evr_command.read() {
                info!("Saying `{}`", ev.command.0);
                nevw_send_chat_msg.broadcast(ServerSendChatMessageEvent {
                    sender: None,
                    message: ev.command.0.clone(),
                });
            }
        },
    );
}
