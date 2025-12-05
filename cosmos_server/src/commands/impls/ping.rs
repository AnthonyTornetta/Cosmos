use bevy::prelude::*;

use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;

struct PingCommand;

impl CosmosCommandType for PingCommand {
    fn from_input(_: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        Ok(PingCommand)
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<PingCommand, _>(
        ServerCommand::new("cosmos:ping", "", "Pong!"),
        app,
        |mut evr_command: MessageReader<CommandMessage<PingCommand>>, mut evw_send_message: MessageWriter<SendCommandMessageMessage>| {
            for ev in evr_command.read() {
                ev.sender.write("Pong!", &mut evw_send_message);
            }
        },
    );
}
