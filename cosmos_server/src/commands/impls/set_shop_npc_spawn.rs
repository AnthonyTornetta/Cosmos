use bevy::prelude::*;
use cosmos_core::prelude::{Ship, Station};

use crate::{
    commands::SendCommandMessageMessage,
    shop::shop_npc::spawn::{ShopNpcSpawnPoint, ShopNpcSpawnPoints},
};

use super::super::prelude::*;

struct SetShopNpcSpawnCommand;

impl CosmosCommandType for SetShopNpcSpawnCommand {
    fn from_input(ev: &crate::commands::CosmosCommandSent) -> Result<Self, ArgumentError> {
        if ev.args.len() > 0 {
            return Err(ArgumentError::TooManyArguments);
        }

        Ok(Self)
    }
}

pub(super) fn register(app: &mut App) {
    create_cosmos_command::<SetShopNpcSpawnCommand, _>(
        ServerCommand::new(
            "cosmos:set_shop_npc_spawn",
            "",
            "Sets a possible location for an NPC spawnpoint where you are standing. You must be a child of this structure.",
        ),
        app,
        |mut evr_blueprint: MessageReader<CommandMessage<SetShopNpcSpawnCommand>>,
         mut evw_send_message: MessageWriter<SendCommandMessageMessage>,
         q_sender: Query<(&Transform, &ChildOf)>,
         mut q_structure: Query<(Entity, Option<&mut ShopNpcSpawnPoints>), Or<(With<Station>, With<Ship>)>>,
         mut commands: Commands| {
            for ev in evr_blueprint.read() {
                let Some((transform, child_of)) = ev.sender.entity().and_then(|e| q_sender.get(e).ok()) else {
                    ev.sender.write(
                        "You must be a player and a child of a structure to use this command!".to_string(),
                        &mut evw_send_message,
                    );
                    continue;
                };

                let Ok((ent, spawn_pts)) = q_structure.get_mut(child_of.parent()) else {
                    ev.sender
                        .write("Invalid structure - must be station or ship.".to_string(), &mut evw_send_message);
                    continue;
                };

                let pt = ShopNpcSpawnPoint {
                    rotation: transform.rotation,
                    relative_position: transform.translation,
                };
                if let Some(mut spawn_pts) = spawn_pts {
                    spawn_pts.add(pt);
                } else {
                    commands.entity(ent).insert(ShopNpcSpawnPoints::new(pt));
                }

                ev.sender.write("Adding point!", &mut evw_send_message);
            }
        },
    );
}
