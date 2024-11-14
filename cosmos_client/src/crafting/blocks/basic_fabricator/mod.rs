use bevy::{
    app::Update,
    core::Name,
    prelude::{App, Commands, Component, Entity, EventReader, IntoSystemConfigs, IntoSystemSetConfigs, Query, SystemSet, With},
    reflect::Reflect,
};
use cosmos_core::{
    crafting::blocks::basic_fabricator::OpenBasicFabricatorEvent,
    ecs::NeedsDespawned,
    netty::{sync::events::client_event::NettyEventReceived, system_sets::NetworkingSystemsSet},
    prelude::StructureBlock,
};

mod ui;

#[derive(Component, Debug, Reflect)]
struct OpenBasicFabricatorMenu(StructureBlock);

fn open_menu(
    q_open_menu: Query<Entity, With<OpenBasicFabricatorMenu>>,
    mut commands: Commands,
    mut nevr: EventReader<NettyEventReceived<OpenBasicFabricatorEvent>>,
) {
    let Some(ev) = nevr.read().last() else {
        return;
    };

    if let Ok(ent) = q_open_menu.get_single() {
        commands.entity(ent).insert(NeedsDespawned);
    }

    commands.spawn((OpenBasicFabricatorMenu(ev.0), Name::new("Open Basic Fabricator Menu")));
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
        open_menu.in_set(NetworkingSystemsSet::Between).in_set(FabricatorMenuSet::OpenMenu),
    )
    .register_type::<OpenBasicFabricatorMenu>();
}
