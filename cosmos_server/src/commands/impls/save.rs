use bevy::prelude::*;

use crate::{commands::SendCommandMessageMessage, persistence::autosave::SaveEverything, server::stop::StopServerSet};

use super::super::prelude::*;

struct SaveCommand;

impl CosmosCommandType for SaveCommand {
    fn from_input(input: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if !input.args.is_empty() {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(SaveCommand)
    }
}

fn send_save_server(
    mut evw_save_server: MessageWriter<SaveEverything>,
    mut evr_command: MessageReader<CommandMessage<SaveCommand>>,
    mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
) {
    let Some(ev) = evr_command.read().next() else {
        return;
    };

    evw_save_server.write_default();
    ev.sender.write("Saving", &mut evw_send_message);
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SaveCommand, _>(
        ServerCommand::new("cosmos:save", "", "Performs a world save"),
        app,
        send_save_server.before(StopServerSet::Stop),
    );
}
