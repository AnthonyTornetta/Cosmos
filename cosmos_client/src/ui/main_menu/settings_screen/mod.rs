use bevy::{app::App, prelude::*};
use cosmos_core::registry::Registry;

use crate::{
    settings::{Setting, SettingData, SettingsSet},
    ui::{
        reactivity::ReactableValue,
        settings::{NeedsSettingsAdded, SettingsCancelButtonEvent, SettingsDoneButtonEvent},
    },
};

use super::{
    disconnect_screen::DisconnectMenuSet, in_main_menu_state, title_screen::TitleScreenSet, MainMenuRootUiNode, MainMenuSubState,
    MainMenuSystemSet,
};

#[derive(Debug, Clone, PartialEq, Eq, Component)]
struct WrittenSetting {
    value: String,
    setting_id: u16,
}

impl ReactableValue for WrittenSetting {
    fn as_value(&self) -> String {
        self.value.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.value = new_value.to_owned()
    }
}

fn create_settings_screen(mut commands: Commands, q_ui_root: Query<Entity, With<MainMenuRootUiNode>>) {
    let Ok(main_menu_root) = q_ui_root.get_single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).insert(NeedsSettingsAdded);
}

fn cancel_clicked(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::TitleScreen;
}

fn done_clicked(mut mms: ResMut<MainMenuSubState>, mut settings: ResMut<Registry<Setting>>, q_written_settings: Query<&WrittenSetting>) {
    for written_setting in q_written_settings.iter() {
        let setting = settings.from_numeric_id_mut(written_setting.setting_id);

        match setting.data {
            SettingData::F32(_) => {
                let Ok(parsed) = written_setting.value.parse::<f32>() else {
                    continue;
                };

                setting.data = SettingData::F32(parsed);
            }
            SettingData::String(_) => {
                setting.data = SettingData::String(written_setting.value.clone());
            }
        }
    }

    *mms = MainMenuSubState::TitleScreen;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub(super) enum SettingsMenuSet {
    SettingsMenuInteractions,
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        SettingsMenuSet::SettingsMenuInteractions
            .ambiguous_with(DisconnectMenuSet::DisconnectMenuInteractions)
            .ambiguous_with(TitleScreenSet::TitleScreenInteractions),
    );

    app.add_systems(
        Update,
        (
            create_settings_screen
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .run_if(resource_exists_and_changed::<MainMenuSubState>)
                .in_set(MainMenuSystemSet::InitializeMenu),
            cancel_clicked
                .run_if(on_event::<SettingsCancelButtonEvent>())
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .in_set(MainMenuSystemSet::UpdateMenu),
            done_clicked
                .run_if(on_event::<SettingsDoneButtonEvent>())
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .in_set(MainMenuSystemSet::UpdateMenu)
                .in_set(SettingsSet::ChangeSettings),
        )
            .in_set(SettingsMenuSet::SettingsMenuInteractions)
            .chain(),
    );
}
