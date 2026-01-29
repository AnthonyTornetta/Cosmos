use crate::{commands::SendCommandMessageMessage, netty::player_filtering::PlayerWhitelist};

use super::super::prelude::*;
use bevy::prelude::*;
use steamworks::SteamId;

struct WhitelistCommand {
    steam_id: SteamId,
}

impl CosmosCommandType for WhitelistCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        if ev.args.is_empty() {
            return Err(ArgumentError::TooFewArguments);
        }

        let Ok(steam_id) = ev.args[0].parse::<u64>() else {
            return Err(ArgumentError::InvalidType {
                arg_index: 0,
                type_name: "SteamId".into(),
            });
        };
        let steam_id = SteamId::from_raw(steam_id);
        Ok(WhitelistCommand { steam_id })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<WhitelistCommand, _>(
        ServerCommand::new("cosmos:whitelist", "[steam id]", "Adds this steam id to the server's whitelist"),
        app,
        |mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         mut whitelist: Option<ResMut<PlayerWhitelist>>,
         mut commands: Commands,
         mut evr_command: MessageReader<CommandMessage<WhitelistCommand>>| {
            for ev in evr_command.read() {
                if let Some(whitelist) = whitelist.as_mut() {
                    whitelist.add_player(ev.command.steam_id);
                } else {
                    let mut whitelist = PlayerWhitelist::default();
                    whitelist.add_player(ev.command.steam_id);
                    commands.insert_resource(whitelist);
                }

                ev.sender
                    .write(format!("Added {:?} to whitelist.", ev.command.steam_id), &mut evw_send_message);
            }
        },
    );
}
