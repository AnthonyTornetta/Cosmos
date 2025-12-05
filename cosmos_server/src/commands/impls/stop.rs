use bevy::prelude::*;

use crate::{
    commands::SendCommandMessageMessage,
    server::stop::{StopServerMessage, StopServerSet},
};

use super::super::prelude::*;

struct StopCommand;

impl CosmosCommandType for StopCommand {
    fn from_input(input: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if !input.args.is_empty() {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(StopCommand)
    }
}

fn send_stop_server_event(
    mut evw_stop_server: MessageWriter<StopServerMessage>,
    mut evr_command: MessageReader<CommandMessage<StopCommand>>,
    mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
) {
    let Some(ev) = evr_command.read().next() else {
        return;
    };

    evw_stop_server.write_default();
    ev.sender.write("Arrivederci!", &mut evw_send_message);
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<StopCommand, _>(
        ServerCommand::new("cosmos:stop", "", "Stops the server"),
        app,
        send_stop_server_event.before(StopServerSet::Stop),
    );
}
