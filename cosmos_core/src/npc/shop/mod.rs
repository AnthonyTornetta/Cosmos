use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, LockedAxes, ReadMassProperties, RigidBody};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    ecs::sets::FixedUpdateSet,
    entities::EntityId,
    faction::FactionId,
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
        sync_component,
    },
    npc::Npc,
    physics::location::Location,
};

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[require(Npc)]
pub struct ShopNpc;

impl IdentifiableComponent for ShopNpc {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shop_npc"
    }
}

impl SyncableComponent for ShopNpc {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Serialize, Deserialize, Component, Clone, Debug, Reflect, Default)]
pub struct Bounties(Vec<Bounty>);

impl Bounties {
    pub fn add(&mut self, bounty: Bounty) {
        self.0.push(bounty);
    }

    pub fn remove(&mut self, bounty: &Bounty) {
        if let Some((idx, _)) = self.0.iter().enumerate().find(|(_, b)| b.id == bounty.id) {
            self.0.swap_remove(idx);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Reflect)]
pub struct Bounty {
    id: Uuid,
    kind: BountyKind,
    payout: u32,
    relations_increase: u32,
    difficulty: Option<BountyDifficulty>,
    location: Location,
    description: String,
}

impl Bounty {
    pub fn new(
        kind: BountyKind,
        payout: u32,
        relations_increase: u32,
        difficulty: Option<BountyDifficulty>,
        location: Location,
        description: String,
    ) -> Self {
        Self {
            description,
            kind,
            payout,
            relations_increase,
            difficulty,
            location,
            id: Uuid::new_v4(),
        }
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn payout(&self) -> u32 {
        self.payout
    }

    pub fn difficulty(&self) -> Option<BountyDifficulty> {
        self.difficulty
    }

    pub fn relations_increase(&self) -> u32 {
        self.relations_increase
    }

    pub fn location(&self) -> Location {
        self.location
    }

    pub fn kind(&self) -> &BountyKind {
        &self.kind
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Reflect)]
pub enum BountyDifficulty {
    Easy,
    Medium,
    Hard,
    Insane,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Reflect)]
pub enum BountyKind {
    Pirate {
        n_pirates: u32,
    },
    /// This bounty is placed on another faction
    Faction {
        other_faction: FactionId,
        relations_decrease: u32,
    },
    Player {
        id: EntityId,
    },
}

fn setup_shop_npc(mut commands: Commands, q_added_shopnpc: Query<Entity, Added<ShopNpc>>) {
    for e in q_added_shopnpc.iter() {
        commands.entity(e).insert((
            Name::new("Shop NPC"),
            LockedAxes::ROTATION_LOCKED,
            RigidBody::KinematicVelocityBased,
            Collider::capsule_y(0.65, 0.25),
            ReadMassProperties::default(),
        ));
    }
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub struct ChatWithShopNpcMessage {
    pub npc: Entity,
}

impl IdentifiableMessage for ChatWithShopNpcMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:chat_with_shop_npc"
    }
}

impl NettyMessage for ChatWithShopNpcMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        let e = mapping.server_from_client(&self.npc)?;
        Some(Self { npc: e })
    }
}

#[derive(Message, Serialize, Deserialize, Clone, Debug)]
pub struct ShopNpcDialogOptions {
    pub text: String,
    pub entity: Entity,
    pub bounties: Bounties,
}

impl IdentifiableMessage for ShopNpcDialogOptions {
    fn unlocalized_name() -> &'static str {
        "cosmos:shop_npc_dialog_options"
    }
}

impl NettyMessage for ShopNpcDialogOptions {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        let entity = mapping.client_from_server(&self.entity)?;
        Some(Self { entity, ..self })
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<ShopNpc>(app);

    app.add_systems(FixedUpdate, setup_shop_npc.in_set(FixedUpdateSet::Main))
        .add_netty_message::<ChatWithShopNpcMessage>()
        .add_netty_message::<ShopNpcDialogOptions>();
}
