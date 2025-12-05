use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::{chat::ServerSendChatMessageMessage, netty::sync::events::server_event::NettyMessageWriter};

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
        |mut nevw_send_chat_msg: NettyMessageWriter<ServerSendChatMessageMessage>,
         mut evr_command: MessageReader<CommandMessage<SayCommand>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>| {
            for ev in evr_command.read() {
                ev.sender.write(format!("Saying `{}`", ev.command.0), &mut evw_send_message);
                nevw_send_chat_msg.broadcast(ServerSendChatMessageMessage {
                    sender: None,
                    message: ev.command.0.clone(),
                });
            }
        },
    );
}
