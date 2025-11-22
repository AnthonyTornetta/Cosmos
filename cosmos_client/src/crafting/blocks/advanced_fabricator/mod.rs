use bevy::prelude::*;
use cosmos_core::{
    crafting::blocks::advanced_fabricator::OpenAdvancedFabricatorMessage,
    ecs::NeedsDespawned,
    netty::sync::{
        events::client_event::NettyMessageReceived,
        mapping::{Mappable, NetworkMapping},
    },
    prelude::StructureBlock,
    state::GameState,
};

mod ui;

#[derive(Component, Debug, Reflect)]
struct OpenAdvancedFabricatorMenu(StructureBlock);

fn open_menu(
    q_open_menu: Query<Entity, With<OpenAdvancedFabricatorMenu>>,
    mut commands: Commands,
    mut nevr: MessageReader<NettyMessageReceived<OpenAdvancedFabricatorMessage>>,
    network_mapping: Res<NetworkMapping>,
) {
    let Some(ev) = nevr.read().last() else {
        return;
    };

    if let Ok(ent) = q_open_menu.single() {
        commands.entity(ent).insert(NeedsDespawned);
    }

    let Ok(s_block) = ev.0.map_to_client(&network_mapping) else {
        error!("Bad network mapping - {:?}", ev.0);
        return;
    };

    commands.spawn((OpenAdvancedFabricatorMenu(s_block), Name::new("Open Advanced Fabricator Menu")));
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum FabricatorMenuSet {
    OpenMenu,
    PopulateMenu,
}

pub(super) fn register(app: &mut App) {
    ui::register(app);

    app.configure_sets(Update, (FabricatorMenuSet::OpenMenu, FabricatorMenuSet::PopulateMenu).chain());

    app.add_systems(
        Update,
        open_menu.in_set(FabricatorMenuSet::OpenMenu).run_if(in_state(GameState::Playing)),
    )
    .register_type::<OpenAdvancedFabricatorMenu>();
}
