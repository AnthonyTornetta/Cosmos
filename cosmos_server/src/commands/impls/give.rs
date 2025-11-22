use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    inventory::{Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    registry::Registry,
};

use super::super::prelude::*;

struct GiveCommand {
    player: String,
    item: String,
    quantity: u16,
}

impl CosmosCommandType for GiveCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        let mut args_iter = ev.args.iter();

        let (Some(player_name), Some(item_id), quantity) = (args_iter.next(), args_iter.next(), args_iter.next()) else {
            return Err(ArgumentError::TooFewArguments);
        };

        if args_iter.next().is_some() {
            return Err(ArgumentError::TooManyArguments);
        }

        let quantity = quantity.map(|x| x.parse::<u16>()).unwrap_or(Ok(1));

        let quantity = match quantity {
            Ok(x) => x,
            Err(_) => {
                return Err(ArgumentError::InvalidType {
                    arg_index: 2,
                    type_name: "u16".into(),
                });
            }
        };

        Ok(Self {
            quantity,
            item: item_id.clone(),
            player: player_name.clone(),
        })
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<GiveCommand, _>(
        ServerCommand::new(
            "cosmos:give",
            "[player] [item_id] (quantity)",
            "Gives the player that item with the specified quantity",
        ),
        app,
        |mut evr_blueprint: MessageReader<CommandMessage<GiveCommand>>,
         mut q_inventory: Query<(&Player, &mut Inventory)>,
         items: Res<Registry<Item>>,
         mut commands: Commands,
         needs_data: Res<ItemShouldHaveData>| {
            for ev in evr_blueprint.read() {
                let player_name = &ev.command.player;
                let item_id = &ev.command.item;
                let quantity = ev.command.quantity;

                let Some((_, mut player_inventory)) = q_inventory.iter_mut().find(|(player, _)| player.name() == player_name) else {
                    println!("Unable to find player {player_name}");
                    continue;
                };

                let mut item_id = item_id.to_owned();

                if !item_id.contains(":") {
                    item_id = format!("cosmos:{item_id}");
                }

                let Some(item) = items.from_id(&item_id) else {
                    println!("Unable to find item {item_id}.");
                    continue;
                };

                let (leftover, _) = player_inventory.insert_item(item, quantity, &mut commands, &needs_data);

                if leftover == 0 {
                    println!("Gave {player_name} {quantity}x {item_id}");
                } else {
                    println!(
                        "Gave {player_name} {}x {item_id}. Inventory could not fit {leftover} item(s).",
                        quantity - leftover
                    );
                }
            }
        },
    );
}
