use bevy::{color::palettes::css, ecs::relationship::RelatedSpawnerCommands, prelude::*};
use bevy_renet::steam::steamworks::{FriendFlags, FriendState};
use cosmos_core::{ecs::NeedsDespawned, state::GameState};
use steamworks::Friend;

use crate::{
    netty::{connect::ConnectToConfig, steam::User},
    ui::{
        OpenMenu,
        components::{
            button::{ButtonEvent, CosmosButton},
            scollable_container::ScrollBox,
            show_cursor::ShowCursor,
            text_input::TextInput,
        },
        constants::CROSS,
        font::DefaultFont,
        reactivity::{BindValue, BindValues, ReactableFields, ReactableValue, add_reactable_type},
    },
};

#[derive(Component)]
#[require(Node)]
/// Use the [`InviteFriendsUiBundle`] bundle instead of directly adding this component
pub struct InviteFriendsUi;

#[derive(Bundle)]
/// Spawns the UI components needed to invite someone to the game
pub struct InviteFriendsUiBundle {
    node: Node,
    background_color: BackgroundColor,
    invite_friends_ui: InviteFriendsUi,
    global_z_index: GlobalZIndex,
    open_menu: OpenMenu,
    show_cursor: ShowCursor,
}

impl Default for InviteFriendsUiBundle {
    fn default() -> Self {
        Self {
            node: Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                ..Default::default()
            },
            background_color: BackgroundColor(Srgba::hex("000000aa").unwrap().into()),
            invite_friends_ui: InviteFriendsUi,
            global_z_index: GlobalZIndex(1000),
            open_menu: OpenMenu::new(10),
            show_cursor: ShowCursor,
        }
    }
}

fn is_offline(state: FriendState) -> bool {
    matches!(
        state,
        FriendState::Busy | FriendState::Away | FriendState::Offline | FriendState::Snooze
    )
}

fn remove_invite_friends_ui(mut commands: Commands, mut removed_friends_ui: RemovedComponents<InviteFriendsUi>) {
    for e in removed_friends_ui.read() {
        commands.entity(e).try_insert(NeedsDespawned);
    }
}

#[derive(Component, Clone, PartialEq, Eq)]
struct FriendSearch(String);

impl ReactableValue for FriendSearch {
    fn as_value(&self) -> String {
        self.0.clone()
    }

    fn set_from_value(&mut self, new_value: &str) {
        self.0 = new_value.into();
    }
}

fn on_add_invite_friends_ui(
    mut commands: Commands,
    mut q_added: Query<Entity, Added<InviteFriendsUi>>,
    steam_user: Res<User>,
    font: Res<DefaultFont>,
) {
    for ent in q_added.iter_mut() {
        commands.entity(ent).insert(FriendSearch("".into())).with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Px(600.0),
                    height: Val::Percent(80.0),
                    margin: UiRect::AUTO,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    ..Default::default()
                },
                BorderColor::all(css::AQUA),
                BackgroundColor(Srgba::hex("333333").unwrap().into()),
            ))
            .with_children(|p| {
                p.spawn((
                    BackgroundColor(Srgba::hex("111111").unwrap().into()),
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        ..Default::default()
                    },
                ))
                .with_children(|p| {
                    p.spawn((
                        Node {
                            margin: UiRect {
                                left: Val::Px(10.0),
                                top: Val::Auto,
                                bottom: Val::Auto,
                                right: Val::Px(10.0),
                            },
                            flex_grow: 1.0,
                            ..Default::default()
                        },
                        TextFont {
                            font: font.get(),
                            font_size: 32.0,
                            ..Default::default()
                        },
                        Text::new("Invite Friends"),
                    ));

                    p.spawn((
                        CosmosButton {
                            text: Some((
                                CROSS.into(),
                                TextFont {
                                    font: font.get(),
                                    font_size: 32.0,
                                    ..Default::default()
                                },
                                Default::default(),
                            )),
                            ..Default::default()
                        },
                        Node {
                            margin: UiRect::all(Val::Px(5.0)),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..Default::default()
                        },
                    ))
                    .observe(move |_: On<ButtonEvent>, mut commands: Commands| {
                        commands.entity(ent).remove::<InviteFriendsUi>();
                    });
                });

                p.spawn((
                    Node {
                        width: Val::Auto,
                        margin: UiRect {
                            left: Val::Px(10.0),
                            right: Val::Px(10.0),
                            top: Val::Px(10.0),
                            bottom: Val::Px(10.0),
                        },
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    TextInput { ..Default::default() },
                    BindValues::single(BindValue::<FriendSearch>::new(ent, ReactableFields::Value)),
                    BackgroundColor(Srgba::hex("00000066").unwrap().into()),
                    BorderColor::all(css::WHITE),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                ));

                p.spawn((
                    Node {
                        flex_grow: 1.0,
                        ..Default::default()
                    },
                    ScrollBox { ..Default::default() },
                ))
                .with_children(|p| {
                    let friends = steam_user.client().friends().get_friends(FriendFlags::ALL);
                    let playing = friends
                        .iter()
                        .filter(|x| {
                            x.game_played()
                                .map_or(false, |x| x.game.app_id() == steam_user.client().utils().app_id())
                        })
                        .collect::<Vec<_>>();
                    let online = friends
                        .iter()
                        .filter(|x| !is_offline(x.state()) && !playing.iter().any(|p| p.id() == x.id()))
                        .collect::<Vec<_>>();
                    let offline = friends.iter().filter(|x| is_offline(x.state())).collect::<Vec<_>>();

                    add_friends_to_ui(ent, p, &playing, "In-Game", &font);
                    add_friends_to_ui(ent, p, &online, "Online", &font);
                    add_friends_to_ui(ent, p, &offline, "Offline", &font);
                });
            });
        });
    }
}

#[derive(Component)]
struct Filter {
    nick: Option<String>,
    name: String,
}

fn on_change_search_term(q_text: Query<&FriendSearch, Changed<FriendSearch>>, mut q_filterable: Query<(&mut Node, &Filter)>) {
    let Ok(search) = q_text.single() else {
        return;
    };

    let search = search.0.trim().to_lowercase();

    for (mut node, filter) in q_filterable.iter_mut() {
        node.display = if filter.name.to_lowercase().contains(&search) || filter.nick.as_ref().map_or(false, |n| n.contains(&search)) {
            Display::default()
        } else {
            Display::None
        };
    }
}

fn add_friends_to_ui(main_ui_ent: Entity, p: &mut RelatedSpawnerCommands<ChildOf>, playing: &[&Friend], title: &str, font: &DefaultFont) {
    if !playing.is_empty() {
        p.spawn((
            Text::new(title),
            Node {
                margin: UiRect::all(Val::Px(4.0)),
                ..Default::default()
            },
            TextFont {
                font_size: 16.0,
                font: font.get(),
                ..Default::default()
            },
        ));
        for friend in playing {
            let name = friend.nick_name().unwrap_or(friend.name());
            let steam_id = friend.id();
            p.spawn((
                Filter {
                    nick: friend.nick_name(),
                    name: friend.name(),
                },
                BorderColor::all(css::AQUA),
                Node {
                    border: UiRect::all(Val::Px(1.0)),
                    width: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    ..Default::default()
                },
                CosmosButton { ..Default::default() },
            ))
            .with_children(|p| {
                p.spawn((
                    Text::new(name),
                    TextFont {
                        font: font.get(),
                        font_size: 24.0,
                        ..Default::default()
                    },
                    Pickable {
                        is_hoverable: false,
                        should_block_lower: false,
                    },
                ));
            })
            .observe(
                move |_: On<ButtonEvent>, steam_user: Res<User>, host_config: Res<ConnectToConfig>, mut commands: Commands| {
                    let connection_config = serde_json::to_string(host_config.as_ref()).unwrap();
                    let friend = steam_user.client().friends().get_friend(steam_id);
                    friend.invite_user_to_game(&connection_config);

                    commands.entity(main_ui_ent).remove::<InviteFriendsUi>();
                },
            );
        }
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<FriendSearch>(app);

    app.add_systems(
        Update,
        (on_add_invite_friends_ui, on_change_search_term, remove_invite_friends_ui).run_if(in_state(GameState::Playing)),
    );
}
