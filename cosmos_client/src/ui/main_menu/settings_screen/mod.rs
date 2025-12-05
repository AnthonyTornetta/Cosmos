use bevy::{app::App, prelude::*};

use crate::{
    settings::SettingsSet,
    ui::{
        components::button::ButtonUiSystemSet,
        settings::{NeedsSettingsAdded, SettingsCancelButtonMessage, SettingsDoneButtonMessage, SettingsMenuSet},
    },
};

use super::{
    MainMenuRootUiNode, MainMenuSubState, MainMenuSystemSet, disconnect_screen::DisconnectMenuSet, in_main_menu_state,
    title_screen::TitleScreenSet,
};

fn create_settings_screen(mut commands: Commands, q_ui_root: Query<Entity, With<MainMenuRootUiNode>>) {
    let Ok(main_menu_root) = q_ui_root.single() else {
        warn!("No main menu UI root.");
        return;
    };

    commands.entity(main_menu_root).insert(NeedsSettingsAdded);
}

fn cancel_clicked(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::TitleScreen;
}

fn done_clicked(mut mms: ResMut<MainMenuSubState>) {
    *mms = MainMenuSubState::TitleScreen;
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        SettingsMenuSet::SettingsMenuInteractions
            .ambiguous_with(DisconnectMenuSet::DisconnectMenuInteractions)
            .ambiguous_with(TitleScreenSet::TitleScreenInteractions)
            .before(SettingsSet::ChangeSettings)
            .after(ButtonUiSystemSet::SendButtonMessages),
    );

    app.add_systems(
        Update,
        (
            create_settings_screen
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .run_if(resource_exists_and_changed::<MainMenuSubState>)
                .in_set(MainMenuSystemSet::InitializeMenu)
                .before(SettingsMenuSet::SettingsMenuInteractions),
            cancel_clicked
                .run_if(on_message::<SettingsCancelButtonMessage>)
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .in_set(MainMenuSystemSet::UpdateMenu)
                .after(SettingsMenuSet::SettingsMenuInteractions),
            done_clicked
                .run_if(on_message::<SettingsDoneButtonMessage>)
                .run_if(in_main_menu_state(MainMenuSubState::Settings))
                .in_set(MainMenuSystemSet::UpdateMenu)
                .after(SettingsSet::ChangeSettings)
                .after(SettingsMenuSet::SettingsMenuInteractions),
        )
            .chain(),
    );
}
