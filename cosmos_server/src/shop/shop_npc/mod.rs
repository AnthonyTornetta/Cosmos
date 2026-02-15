use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    entities::{EntityId, player::Player},
    faction::{FactionId, FactionRelation, Factions},
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyMessageReceived, NettyMessageWriter},
    },
    npc::shop::{Bounties, Bounty, BountyDifficulty, BountyKind, ChatWithShopNpcMessage, ShopNpc, ShopNpcDialogOptions},
    physics::location::Location,
    state::in_gameplay_state,
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

pub mod spawn;

impl DefaultPersistentComponent for ShopNpc {}

const MIN_SPAWN_SIZE: f32 = 20_000.0;

fn on_talk_to_shop_npc(
    mut nmr_talk_to_npc: MessageReader<NettyMessageReceived<ChatWithShopNpcMessage>>,
    factions: Res<Factions>,
    q_npc: Query<(Option<&FactionId>, &Location), With<ShopNpc>>,
    q_player: Query<(&EntityId, &Player, Option<&FactionId>), With<Player>>,
    lobby: Res<ServerLobby>,
    mut nmw_npc_response: NettyMessageWriter<ShopNpcDialogOptions>,
) {
    for msg in nmr_talk_to_npc.read() {
        let Some((player_id, player, player_fac)) = lobby.player_from_id(msg.client_id).and_then(|e| q_player.get(e).ok()) else {
            continue;
        };

        let Ok((shop_fac, location)) = q_npc.get(msg.npc) else {
            continue;
        };

        let player_fac = player_fac.and_then(|f| factions.from_id(f));
        let shop_fac = shop_fac.and_then(|f| factions.from_id(f));

        let relation = shop_fac.map(|f| f.relation_with_entity(player_id, player_fac)).unwrap_or_default();

        let text = match relation {
            FactionRelation::Ally => format!("Greetings {}! How can I help?", player.name()),
            FactionRelation::Neutral => "Hello traveller! How can I help?".into(),
            FactionRelation::Enemy => continue, // can't talk to enemy shopkeeper
        };

        let mut bounties = Bounties::default();

        let offset = Vec3::new(
            rand::random::<f32>() * 10_000.0,
            rand::random::<f32>() * 10_000.0,
            rand::random::<f32>() * 10_000.0,
        ) + Vec3::splat(MIN_SPAWN_SIZE);

        bounties.add(Bounty::new(
            BountyKind::Pirate { n_pirates: 1 },
            100_000,
            5,
            Some(BountyDifficulty::Easy),
            *location + offset,
            "Deliver some cosmic justice to some pirates!".into(),
        ));

        nmw_npc_response.write(
            ShopNpcDialogOptions {
                entity: msg.npc,
                text,
                bounties,
            },
            msg.client_id,
        );
    }
}

pub(super) fn register(app: &mut App) {
    spawn::register(app);
    make_persistent::<ShopNpc>(app);

    app.add_systems(
        FixedUpdate,
        on_talk_to_shop_npc.run_if(in_gameplay_state).in_set(FixedUpdateSet::Main),
    );
}
