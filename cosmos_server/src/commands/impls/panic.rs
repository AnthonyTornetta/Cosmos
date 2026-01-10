use bevy::prelude::*;
use cosmos_core::entities::player::Player;

use super::super::prelude::*;

struct PanicCommand;

impl CosmosCommandType for PanicCommand {
    fn from_input(_: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        Ok(PanicCommand)
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<PanicCommand, _>(
        ServerCommand::new(
            "cosmos:panic",
            "",
            "Causes the server to crash. For testing only - you can corrupt your world using this command.",
        ),
        app,
        |q_player: Query<&Player>, mut evr_command: MessageReader<CommandMessage<PanicCommand>>| {
            let Some(ev) = evr_command.read().next() else {
                return;
            };
            let name = ev
                .sender
                .entity()
                .and_then(|e| q_player.get(e).ok().map(|x| x.name()))
                .unwrap_or("<server console>");

            panic!("Panic command executed by {name}");
        },
    );
}
