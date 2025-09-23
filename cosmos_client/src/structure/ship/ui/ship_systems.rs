use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::{
    netty::sync::events::client_event::NettyEventWriter,
    prelude::{StructureSystem, StructureSystems},
    registry::Registry,
    state::GameState,
    structure::systems::{ChangeSystemSlot, StructureSystemId, StructureSystemOrdering, StructureSystemType},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    lang::Lang,
    ui::{
        components::{
            button::{ButtonEvent, CosmosButton},
            scollable_container::ScrollBox,
        },
        font::DefaultFont,
        item_renderer::{CustomHoverTooltip, RenderItem},
    },
};

#[derive(Component, Debug)]
#[require(Node)]
pub struct ShipSystemsUi {
    ship_ent: Entity,
}

impl ShipSystemsUi {
    pub fn new(ship_ent: Entity) -> Self {
        Self { ship_ent }
    }
}

#[derive(Component)]
struct StructureSystemMarker {
    system_id: StructureSystemId,
    structure: Entity,
}

fn on_change_structure_systems(
    q_changed_ship_systems: Query<
        (Entity, &StructureSystems, &StructureSystemOrdering),
        Or<(Changed<StructureSystems>, Changed<StructureSystemOrdering>)>,
    >,
    q_structure_system: Query<&StructureSystem>,
    system_types: Res<Registry<StructureSystemType>>,
    mut commands: Commands,
    font: Res<DefaultFont>,
    q_system_ui: Query<(Entity, &ShipSystemsUi)>,
    lang: Res<Lang<StructureSystemType>>,
) {
    for (structure_ent, systems, ordering) in q_changed_ship_systems.iter() {
        let Some((ent, ss)) = q_system_ui.iter().find(|(_, x)| x.ship_ent == structure_ent) else {
            continue;
        };

        commands
            .entity(ent)
            .despawn_related::<Children>()
            .remove::<ScrollBox>()
            .insert(ScrollBox::default())
            .with_children(|p| {
                render_ui(p, &font, &q_structure_system, systems, &system_types, ss, ordering, &lang);
            });
    }
}

fn render_ui(
    p: &mut RelatedSpawnerCommands<ChildOf>,
    font: &DefaultFont,
    q_structure_system: &Query<&StructureSystem>,
    systems: &StructureSystems,
    system_types: &Registry<StructureSystemType>,
    ss: &ShipSystemsUi,
    ordering: &StructureSystemOrdering,
    lang: &Lang<StructureSystemType>,
) {
    let n_systems = systems
        .all_activatable_systems()
        .filter(|x| q_structure_system.contains(*x))
        .count();

    if n_systems == 0 {
        p.spawn((
            Node {
                margin: UiRect::all(Val::Px(20.0)),
                ..Default::default()
            },
            TextFont {
                font: font.get(),
                font_size: 24.0,
                ..Default::default()
            },
            Text::new("No Ship Systems!"),
        ));
        return;
    }

    for (system, system_type) in systems
        .all_activatable_systems()
        .flat_map(|x| q_structure_system.get(x))
        .map(|x| (x, system_types.from_numeric_id(x.system_type_id().into())))
    {
        p.spawn((
            CosmosButton { ..Default::default() },
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            StructureSystemMarker {
                system_id: system.id(),
                structure: ss.ship_ent,
            },
        ))
        .observe(on_system_clicked)
        .with_children(|p| {
            let system_name = lang.get_name_or_unlocalized(system_type);
            p.spawn((
                RenderItem {
                    item_id: system_type.item_icon_id(),
                },
                Node {
                    width: Val::Px(100.0),
                    height: Val::Px(100.0),
                    ..Default::default()
                },
                CustomHoverTooltip::new(system_name),
            ));

            let ordering = ordering.ordering_for(system.id());

            p.spawn((
                Node {
                    width: Val::Px(50.0),
                    ..Default::default()
                },
                Text::new(if let Some(ordering) = ordering {
                    format!("{}", ordering + 1)
                } else {
                    " ".into()
                }),
                TextFont {
                    font: font.get(),
                    font_size: 32.0,
                    ..Default::default()
                },
            ));

            p.spawn((
                Node {
                    flex_grow: 1.0,
                    ..Default::default()
                },
                Text::new(system_name),
                TextFont {
                    font: font.get(),
                    font_size: 24.0,
                    ..Default::default()
                },
            ));
        });
    }
}

pub(super) fn attach_ui(
    q_ship_systems: Query<(&StructureSystems, &StructureSystemOrdering)>,
    q_structure_system: Query<&StructureSystem>,
    system_types: Res<Registry<StructureSystemType>>,
    mut commands: Commands,
    q_needs_ship_systems_ui: Query<(Entity, &ShipSystemsUi), Added<ShipSystemsUi>>,
    font: Res<DefaultFont>,
    lang: Res<Lang<StructureSystemType>>,
) {
    for (ent, ss) in q_needs_ship_systems_ui.iter() {
        let Ok((systems, ordering)) = q_ship_systems.get(ss.ship_ent) else {
            continue;
        };

        commands
            .entity(ent)
            .insert((
                Name::new("Ship Systems"),
                ScrollBox { ..Default::default() },
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                render_ui(p, &font, &q_structure_system, systems, &system_types, ss, ordering, &lang);
            });
    }
}

fn on_add_active_system(mut q_active_system: Query<&mut BackgroundColor, Added<ActiveSystem>>) {
    for mut bg in q_active_system.iter_mut() {
        bg.0 = css::AQUA.into();
    }
}

fn on_remove_active_system(mut removed: RemovedComponents<ActiveSystem>, mut q_bg: Query<&mut BackgroundColor>) {
    for ent in removed.read() {
        if let Ok(mut bg) = q_bg.get_mut(ent) {
            *bg = Default::default();
        }
    }
}

#[derive(Component)]
struct ActiveSystem;

fn on_system_clicked(ev: Trigger<ButtonEvent>, mut commands: Commands, q_active: Query<Entity, With<ActiveSystem>>) {
    if let Ok(active) = q_active.single() {
        commands.entity(active).remove::<ActiveSystem>();

        if active != ev.0 {
            commands.entity(ev.0).insert(ActiveSystem);
        }

        return;
    }
    commands.entity(ev.0).insert(ActiveSystem);
}

fn listen_button_inputs(
    q_systems: Query<&StructureSystemOrdering>,
    input_handler: InputChecker,
    q_active_system: Query<(Entity, &StructureSystemMarker), With<ActiveSystem>>,
    mut nevw_change_system_slot: NettyEventWriter<ChangeSystemSlot>,
    mut commands: Commands,
) {
    let Ok((ent, active)) = q_active_system.single() else {
        return;
    };

    let Ok(ordering) = q_systems.get(active.structure) else {
        return;
    };

    if input_handler.check_just_pressed(CosmosInputs::Pause) {
        if let Some((slot, _)) = ordering.iter().enumerate().find(|(_, x)| *x == Some(active.system_id)) {
            nevw_change_system_slot.write(ChangeSystemSlot {
                slot: slot as u32,
                system_id: None,
                structure: active.structure,
            });
        }
        commands.entity(ent).remove::<ActiveSystem>();
        return;
    }

    let slot = if input_handler.check_just_pressed(CosmosInputs::HotbarSlot1) {
        Some(0)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot2) {
        Some(1)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot3) {
        Some(2)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot4) {
        Some(3)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot5) {
        Some(4)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot6) {
        Some(5)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot7) {
        Some(6)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot8) {
        Some(7)
    } else if input_handler.check_just_pressed(CosmosInputs::HotbarSlot9) {
        Some(8)
    } else {
        None
    };

    let Some(slot) = slot else {
        return;
    };

    commands.entity(ent).remove::<ActiveSystem>();

    info!("Sending Event!");
    nevw_change_system_slot.write(ChangeSystemSlot {
        slot,
        system_id: Some(active.system_id),
        structure: active.structure,
    });
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            attach_ui,
            on_change_structure_systems,
            listen_button_inputs,
            on_add_active_system,
            on_remove_active_system,
        )
            .run_if(in_state(GameState::Playing)),
    );
}
