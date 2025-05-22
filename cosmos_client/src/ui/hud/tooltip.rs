use bevy::prelude::*;
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::client::LocalPlayer,
    prelude::{Structure, StructureBlock},
};

use crate::{
    interactions::block_interactions::LookingAt,
    ui::{font::DefaultFont, message::HudMessage},
};

#[derive(Clone, Debug, Reflect)]
struct TooltipMessage {
    message: HudMessage,
    label: String,
    ent: Option<Entity>,
}

#[derive(Component, Clone, Debug, Reflect, Default)]
pub struct LookingAtTooltip {
    info: Vec<TooltipMessage>,
}

impl LookingAtTooltip {
    pub fn add_or_modify_message(&mut self, message: HudMessage, label: &str) {
        if let Some(msg) = self.info.iter_mut().find(|x| x.label == label) {
            msg.message = message;
        } else {
            self.info.push(TooltipMessage {
                label: label.into(),
                message,
                ent: None,
            })
        }
    }
}

#[derive(Event)]
pub struct GenerateLookingAtTooltipEvent {
    pub tooltip_entity: Entity,
    pub looking_at: StructureBlock,
    pub block_id: u16,
}

fn on_change_looking_at(
    mut commands: Commands,
    q_tooltip: Query<Entity, With<LookingAtTooltip>>,
    q_looking_at: Query<&LookingAt, (Changed<LookingAt>, With<LocalPlayer>)>,
    q_structure: Query<&Structure>,
    mut evw_generate_tooltip: EventWriter<GenerateLookingAtTooltipEvent>,
) {
    let Ok(looking_at) = q_looking_at.get_single() else {
        return;
    };

    let Some(block) = looking_at.looking_at_block else {
        if let Ok(tooltip_ent) = q_tooltip.get_single() {
            commands.entity(tooltip_ent).insert(NeedsDespawned);
        }
        return;
    };

    let mut ecmds = if let Ok(tooltip_ent) = q_tooltip.get_single() {
        commands.entity(tooltip_ent)
    } else {
        commands.spawn_empty()
    };

    let Ok(structure) = q_structure.get(block.block.structure()) else {
        return;
    };

    let block_id = structure.block_id_at(block.block.coords());

    let ent = ecmds
        .insert((
            LookingAtTooltip::default(),
            Name::new("Looking at tooltip"),
            Node {
                position_type: PositionType::Absolute,
                margin: UiRect::left(Val::Px(200.0)),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
        ))
        .despawn_descendants()
        .id();

    evw_generate_tooltip.send(GenerateLookingAtTooltipEvent {
        looking_at: block.block,
        tooltip_entity: ent,
        block_id,
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum LookingAtTooltipSet {
    CreateTooltipEntity,
    GenerateTooltipContent,
    ApplyTooltipText,
}

fn on_finish_tooltip_text(
    mut commands: Commands,
    font: Res<DefaultFont>,
    mut q_looking_at: Query<(Entity, &mut LookingAtTooltip), Changed<LookingAtTooltip>>,
) {
    for (ent, mut looking_at_tooltip) in q_looking_at.iter_mut() {
        for message in looking_at_tooltip.info.iter_mut() {
            let mut ecmds = if let Some(ent) = message.ent {
                commands.entity(ent)
            } else {
                let mut ecmds = commands.spawn(Name::new("Tooltip Message"));

                message.ent = Some(ecmds.id());

                ecmds.set_parent(ent);

                ecmds
            };

            let mut text = message.message.iter();

            let Some(first) = text.next() else {
                continue;
            };

            let font = TextFont {
                font: font.0.clone_weak(),
                font_size: 18.0,
                ..Default::default()
            };

            ecmds
                .insert((
                    font.clone(),
                    Text::new(&first.text),
                    TextColor(first.color),
                    TextLayout { ..Default::default() },
                ))
                .with_children(|p| {
                    for next_message in text {
                        p.spawn((font.clone(), TextSpan::new(&next_message.text), TextColor(next_message.color)));
                    }
                });
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<GenerateLookingAtTooltipEvent>();

    app.configure_sets(
        Update,
        (
            LookingAtTooltipSet::CreateTooltipEntity,
            LookingAtTooltipSet::GenerateTooltipContent,
            LookingAtTooltipSet::ApplyTooltipText,
        )
            .chain(),
    );

    app.add_systems(
        Update,
        (
            on_change_looking_at.in_set(LookingAtTooltipSet::CreateTooltipEntity),
            on_finish_tooltip_text.in_set(LookingAtTooltipSet::ApplyTooltipText),
        ),
    );
}
