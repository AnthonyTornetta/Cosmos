use bevy::prelude::*;

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
        |mut evr_command: MessageReader<CommandMessage<PingCommand>>| {
            for _ in evr_command.read() {
                info!("Pong!");
            }
        },
    );
}
