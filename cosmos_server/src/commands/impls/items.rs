use crate::commands::SendCommandMessageMessage;

use super::super::prelude::*;
use bevy::prelude::*;
use cosmos_core::{
    item::Item,
    registry::{Registry, identifiable::Identifiable},
};

struct ItemsCommand(Option<String>);

impl CosmosCommandType for ItemsCommand {
    fn from_input(ev: &CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 1 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(ItemsCommand(ev.args.first().cloned()))
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<ItemsCommand, _>(
        ServerCommand::new("cosmos:items", "(search term)", "Displays all items that match this search term"),
        app,
        |mut evr_command: MessageReader<CommandMessage<ItemsCommand>>,
         items: Res<Registry<Item>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>| {
            for ev in evr_command.read() {
                let search_term = ev.command.0.as_deref().unwrap_or("").to_lowercase();
                let result = items
                    .iter()
                    .filter(|x| x.unlocalized_name().to_lowercase().contains(&search_term))
                    .map(|x| x.unlocalized_name())
                    .collect::<Vec<_>>();

                if result.is_empty() {
                    ev.sender.write("No items found.", &mut evw_send_message);
                } else {
                    ev.sender.write("Items:", &mut evw_send_message);
                    for item in result {
                        ev.sender.write(item, &mut evw_send_message);
                    }
                }
            }
        },
    );
}
