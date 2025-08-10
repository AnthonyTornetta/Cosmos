use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use cosmos_core::{
    faction::{
        Faction, FactionId, Factions,
        events::{FactionSwapAction, SwapToPlayerFactionEvent},
    },
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter},
    state::GameState,
};

use crate::ui::{
    components::button::{ButtonEvent, CosmosButton, register_button},
    font::DefaultFont,
};

#[derive(Component)]
#[require(Node)]
pub(super) struct ShipDetailsUi(Entity);

impl ShipDetailsUi {
    pub fn new(ship_ent: Entity) -> Self {
        Self(ship_ent)
    }
}

#[derive(Component)]
struct FactionButton {
    ship_ent: Entity,
}

#[derive(Event, Debug)]
struct FactionButtonEvent(Entity);

impl ButtonEvent for FactionButtonEvent {
    fn create_event(btn_entity: Entity) -> Self {
        Self(btn_entity)
    }
}

fn render_ui(
    p: &mut RelatedSpawnerCommands<ChildOf>,
    q_faction: &Query<&FactionId>,
    factions: &Factions,
    font: &DefaultFont,
    ship_ent: Entity,
    local_faction: Option<&Faction>,
) {
    let faction = q_faction.get(ship_ent).ok().and_then(|x| factions.from_id(x));

    let faction_text = if let Some(f) = faction {
        format!("Faction: {}", f.name())
    } else {
        "No Faction".to_owned()
    };

    p.spawn(Node {
        flex_grow: 1.0,
        flex_direction: FlexDirection::Column,
        margin: UiRect::all(Val::Px(15.0)),
        ..Default::default()
    })
    .with_children(|p| {
        p.spawn((
            Name::new("Faction Text"),
            Text::new(faction_text),
            TextFont {
                font_size: 24.0,
                font: font.get(),
                ..Default::default()
            },
            Node {
                margin: UiRect::bottom(Val::Px(5.0)),
                ..Default::default()
            },
        ));

        if let Some(local_faction) = local_faction {
            p.spawn((
                Name::new("Faction Button"),
                BackgroundColor(css::DARK_GREY.into()),
                CosmosButton::<FactionButtonEvent> {
                    text: Some((
                        if faction.map(|x| x.id() == local_faction.id()).unwrap_or(false) {
                            "Remove Faction".into()
                        } else {
                            "Set Faction".into()
                        },
                        TextFont {
                            font: font.get(),
                            font_size: 24.0,
                            ..Default::default()
                        },
                        default(),
                    )),
                    ..Default::default()
                },
                Node {
                    width: Val::Px(300.0),
                    height: Val::Px(60.0),
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..Default::default()
                },
                FactionButton { ship_ent },
                TextFont {
                    font_size: 24.0,
                    font: font.get(),
                    ..Default::default()
                },
            ));
        }
    });
}

pub(super) fn attach_ui(
    mut commands: Commands,
    mut q_needs_ship_systems_ui: Query<(Entity, &ShipDetailsUi, &mut Node), Added<ShipDetailsUi>>,
    q_faction: Query<&FactionId>,
    factions: Res<Factions>,
    font: Res<DefaultFont>,
    q_local_faction: Query<&FactionId, With<LocalPlayer>>,
) {
    for (ent, ui, mut node) in q_needs_ship_systems_ui.iter_mut() {
        let local_fac = q_local_faction.single().ok().and_then(|x| factions.from_id(x));

        node.flex_direction = FlexDirection::Column;

        commands
            .entity(ent)
            .insert((Name::new("Ship Details"),))
            .with_children(|p| render_ui(p, &q_faction, &factions, &font, ui.0, local_fac));
    }
}

fn on_change_faction(
    mut evr_change_fac: EventReader<FactionButtonEvent>,
    q_faction_id: Query<(), With<FactionId>>,
    mut nevw_set_faction: NettyEventWriter<SwapToPlayerFactionEvent>,
    q_faction_button: Query<&FactionButton>,
) {
    for ev in evr_change_fac.read() {
        let fac_button = q_faction_button.get(ev.0).unwrap();
        let action = if q_faction_id.contains(fac_button.ship_ent) {
            FactionSwapAction::RemoveFaction
        } else {
            FactionSwapAction::AssignToSelfFaction
        };

        info!("Change faction for {:?} ({action:?})", fac_button.ship_ent);

        nevw_set_faction.write(SwapToPlayerFactionEvent {
            action,
            to_swap: fac_button.ship_ent,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (attach_ui, on_change_faction).run_if(in_state(GameState::Playing)));
    register_button::<FactionButtonEvent>(app);
}
