use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::{persistence::Blueprintable, physics::location::Location};

struct ListCommand;

impl CosmosCommandType for ListCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if !ev.args.is_empty() {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(ListCommand)
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<ListCommand, _>(
        ServerCommand::new("cosmos:list", "", "Lists all the savable entity ids"),
        app,
        |mut evr_command: MessageReader<CommandMessage<ListCommand>>,
         all_blueprintable_entities: Query<(Entity, &Name, &Location), With<Blueprintable>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>| {
            for ev in evr_command.read() {
                ev.sender
                    .write("All blueprintable entities (Name, Sector, Id):", &mut evw_send_message);
                for (entity, name, location) in all_blueprintable_entities.iter() {
                    ev.sender.write(
                        format!("{name}\t{}\t{}", location.sector(), entity.to_bits()),
                        &mut evw_send_message,
                    );
                }
                ev.sender.write("======================================", &mut evw_send_message);
            }
        },
    );
}
