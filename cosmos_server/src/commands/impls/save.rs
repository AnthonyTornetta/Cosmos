use bevy::prelude::*;

use crate::{persistence::autosave::SaveEverything, server::stop::StopServerSet};

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

fn send_save_server(mut evw_stop_server: MessageWriter<SaveEverything>) {
    evw_stop_server.write_default();
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SaveCommand, _>(
        ServerCommand::new("cosmos:save", "", "Performs a world save"),
        app,
        send_save_server.before(StopServerSet::Stop),
    );
}
