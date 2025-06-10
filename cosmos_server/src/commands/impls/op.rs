use crate::commands::{CommandSender, Operator, SendCommandMessageEvent};

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::entities::player::Player;

#[derive(Debug)]
enum Receiver {
    Entity(Entity),
    Name(String),
}

struct OpCommand {
    receiver: Receiver,
}

impl CosmosCommandType for OpCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        let receiver = if ev.args.len() == 1 {
            Receiver::Name(ev.args[0].clone())
        } else {
            match ev.sender {
                CommandSender::Server => return Err(ArgumentError::TooFewArguments),
                CommandSender::Player(e) => Receiver::Entity(e),
            }
        };

        return Ok(OpCommand { receiver });
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<OpCommand, _>(
        ServerCommand::new("cosmos:op", "(player)", "Toggles this player's operator status"),
        app,
        |q_players: Query<(Entity, &Player)>,
         mut commands: Commands,
         mut evw_send_message: EventWriter<SendCommandMessageEvent>,
         q_operator: Query<&Operator>,
         mut evr_command: EventReader<CommandEvent<OpCommand>>| {
            for ev in evr_command.read() {
                let Some((ent, player)) = (match &ev.command.receiver {
                    Receiver::Name(name) => q_players.iter().find(|x| x.1.name() == name),
                    Receiver::Entity(e) => q_players.get(*e).ok(),
                }) else {
                    ev.sender
                        .send(format!("Unable to find player {:?}", ev.command.receiver), &mut evw_send_message);
                    continue;
                };

                if q_operator.contains(ent) {
                    ev.sender.send(format!("De-opped {}.", player.name()), &mut evw_send_message);
                    commands.entity(ent).remove::<Operator>();
                } else {
                    ev.sender.send(format!("Opped {}.", player.name()), &mut evw_send_message);
                    commands.entity(ent).insert(Operator);
                }
            }
        },
    );
}
