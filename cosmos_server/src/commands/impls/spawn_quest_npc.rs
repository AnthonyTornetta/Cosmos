use bevy::prelude::*;
use cosmos_core::{entities::player::Player, physics::location::Location};

use crate::{commands::SendCommandMessageMessage, universe::spawners::quest_npc::MerchantNeedsSpawned};

use super::super::prelude::*;

struct SpawnQuestNpcCommand {
    player: Option<String>,
}

impl CosmosCommandType for SpawnQuestNpcCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(Self {
            player: ev.args.first().cloned(),
        })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SpawnQuestNpcCommand, _>(
        ServerCommand::new(
            "cosmos:spawnquestnpc",
            "(player)",
            "Spawns a quest merchant 500 blocks from the specified player, or yourself if no player is specified.",
        ),
        app,
        |mut evr_command: MessageReader<CommandMessage<SpawnQuestNpcCommand>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         q_players: Query<(Entity, &Player, &Location)>,
         mut commands: Commands| {
            for ev in evr_command.read() {
                let target = if let Some(player_name) = &ev.command.player {
                    q_players
                        .iter()
                        .find(|(_, player, _)| player.name().to_lowercase() == player_name.to_lowercase())
                } else if let Some(sender) = ev.sender.entity() {
                    q_players.get(sender).ok()
                } else {
                    ev.sender.write(
                        "You must specify a player when using this command from the server console!",
                        &mut evw_send_message,
                    );
                    continue;
                };

                let Some((_, player, location)) = target else {
                    ev.sender.write("Unable to find that player!", &mut evw_send_message);
                    continue;
                };

                commands.spawn((
                    Name::new("Loading Merchant Ship"),
                    MerchantNeedsSpawned {
                        location: *location + Vec3::X * 500.0,
                        difficulty: 0,
                        heading_towards: *location,
                    },
                ));

                ev.sender
                    .write(format!("Spawning a quest merchant near {}!", player.name()), &mut evw_send_message);
            }
        },
    );
}
