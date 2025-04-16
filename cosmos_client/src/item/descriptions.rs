//! Descriptions for [`Item`]s.

use std::fs;

use bevy::{prelude::*, text::FontStyle, utils::HashMap};
use cosmos_core::{
    item::Item,
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

#[derive(Debug)]
/// Indicates text that should be rendered with a specific style applied
pub struct StyledText {
    /// The text
    pub text: String,
    /// The color
    pub color: Color,
    /// The style
    pub style: FontStyle,
}

#[derive(Debug)]
/// A description for an item
pub enum ItemDescriptionTextEntry {
    /// Normal, non-interactable text
    Normal(StyledText),
    /// Clicking this will bring you to another [`ItemDescriptionTextEntry`]
    Link {
        /// The text to render
        text: StyledText,
        /// Which entry this text should link to
        to: u16,
    },
}

#[derive(Debug)]
/// A description for an [`Item`]
pub struct ItemDescription(pub Vec<ItemDescriptionTextEntry>);

#[derive(Debug, Default, Resource)]
/// All [`Item`]s mapped to their [`ItemDescription`].
pub struct ItemDescriptions(HashMap<u16, ItemDescription>);

fn load_descriptions(mut descriptions: ResMut<ItemDescriptions>, items: Res<Registry<Item>>) {
    let Ok(lang_file) = fs::read_to_string("assets/cosmos/lang/items/descriptions/en_us.lang") else {
        error!("No lang file to read for descriptions!");
        return;
    };

    for x in lang_file
        .split("\n")
        .map(|x| x.trim())
        .filter(|x| !x.is_empty() && !x.starts_with("#"))
    {
        let mut splt = x.split("=");
        let item_id = splt.next().unwrap();
        let desc = splt.collect::<Vec<_>>().join("=");

        let Some(item) = items.from_id(item_id) else {
            error!("Mising item {item_id} found in lang file");
            continue;
        };

        // TODO: Parse desc
        descriptions.0.insert(
            item.id(),
            ItemDescription(vec![ItemDescriptionTextEntry::Normal(StyledText {
                text: desc,
                color: Color::WHITE,
                style: FontStyle::Normal,
            })]),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<ItemDescriptions>();

    app.add_systems(OnEnter(GameState::PostLoading), load_descriptions);
}
