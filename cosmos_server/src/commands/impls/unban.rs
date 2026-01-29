use crate::{commands::SendCommandMessageMessage, netty::player_filtering::PlayerBlacklist};

use super::super::prelude::*;
use bevy::prelude::*;
use steamworks::SteamId;

struct UnbanCommand {
    receiver: String,
}

impl CosmosCommandType for UnbanCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        let receiver = ev.args[0].clone();

        Ok(UnbanCommand { receiver })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<UnbanCommand, _>(
        ServerCommand::new("cosmos:unban", "[player]", "Unbans this player, allowing them to rejoin the server"),
        app,
        |mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut blacklist: ResMut<PlayerBlacklist>,
         mut evr_command: MessageReader<CommandMessage<UnbanCommand>>| {
            for ev in evr_command.read() {
                if let Ok(id) = ev.command.receiver.parse::<u64>() {
                    let sid = SteamId::from_raw(id);
                    if blacklist.contains_player(&sid) {
                        blacklist.remove_player(&sid);
                        ev.sender.write(format!("Unbanned {id}."), &mut evw_send_message);
                        continue;
                    }
                }

                if let Some(sid) = blacklist.get_player_by_name(&ev.command.receiver) {
                    blacklist.remove_player(&sid);
                    ev.sender
                        .write(format!("Unbanned {} ({sid:?}).", ev.command.receiver), &mut evw_send_message);
                    continue;
                }

                ev.sender
                    .write(format!("`{}` is not banned.", ev.command.receiver), &mut evw_send_message);
            }
        },
    );
}
