use std::fs;

use bevy::{prelude::*, text::FontStyle, utils::HashMap};
use cosmos_core::{
    item::Item,
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

#[derive(Debug)]
pub struct ColoredText {
    pub text: String,
    pub color: Color,
    pub style: FontStyle,
}

#[derive(Debug)]
pub enum ItemDescriptionTextEntry {
    Normal(ColoredText),
    Link { text: ColoredText, to: u16 },
}

#[derive(Debug)]
pub struct ItemDescription(pub Vec<ItemDescriptionTextEntry>);

#[derive(Debug, Default, Resource)]
pub struct ItemDescriptions(HashMap<u16, ItemDescription>);

fn load_descriptions(mut descriptions: ResMut<ItemDescriptions>, items: Res<Registry<Item>>) {
    let Ok(lang_file) = fs::read_to_string("assets/cosmos/lang/items/descriptions/en_us.lang") else {
        error!("No lang file to read for descriptions!");
        return;
    };

    for x in lang_file
        .split("\n")
        .map(|x| x.trim())
        .filter(|x| x.len() != 0 && !x.starts_with("#"))
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
            ItemDescription(vec![ItemDescriptionTextEntry::Normal(ColoredText {
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
