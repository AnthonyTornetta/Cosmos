use bevy::prelude::*;

use crate::server::stop::{StopServerEvent, StopServerSet};

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

fn send_stop_server_event(mut evw_stop_server: EventWriter<StopServerEvent>) {
    evw_stop_server.send_default();
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<StopCommand, _>(
        ServerCommand::new("cosmos:stop", "", "Stops the server"),
        app,
        send_stop_server_event.before(StopServerSet::Stop),
    );
}
