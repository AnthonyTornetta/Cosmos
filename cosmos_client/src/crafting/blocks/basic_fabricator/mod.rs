use bevy::{
    app::Update,
    core::Name,
    log::error,
    prelude::{
        in_state, App, Commands, Component, Entity, EventReader, IntoSystemConfigs, IntoSystemSetConfigs, Query, Res, SystemSet, With,
    },
    reflect::Reflect,
};
use cosmos_core::{
    crafting::blocks::basic_fabricator::OpenBasicFabricatorEvent,
    ecs::NeedsDespawned,
    netty::{
        sync::{
            events::client_event::NettyEventReceived,
            mapping::{Mappable, NetworkMapping},
        },
        system_sets::NetworkingSystemsSet,
    },
    prelude::StructureBlock,
    state::GameState,
};

mod ui;

#[derive(Component, Debug, Reflect)]
struct OpenBasicFabricatorMenu(StructureBlock);

fn open_menu(
    q_open_menu: Query<Entity, With<OpenBasicFabricatorMenu>>,
    mut commands: Commands,
    mut nevr: EventReader<NettyEventReceived<OpenBasicFabricatorEvent>>,
    network_mapping: Res<NetworkMapping>,
) {
    let Some(ev) = nevr.read().last() else {
        return;
    };

    if let Ok(ent) = q_open_menu.get_single() {
        commands.entity(ent).insert(NeedsDespawned);
    }

    let Ok(s_block) = ev.0.map_to_client(&network_mapping) else {
        error!("Bad network mapping - {:?}", ev.0);
        return;
    };

    commands.spawn((OpenBasicFabricatorMenu(s_block), Name::new("Open Basic Fabricator Menu")));
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
        open_menu
            .in_set(NetworkingSystemsSet::Between)
            .in_set(FabricatorMenuSet::OpenMenu)
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<OpenBasicFabricatorMenu>();
}
