use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use sickle_ui::prelude::*;

use crate::chunk_manager::SaveAllChunks;
use crate::player::PlayerSettings;
use crate::widgets::button::{ButtonPressed, ButtonWidgetExt};
use crate::widgets::game_settings::{ApplyGameSettings, AutoApplySettings, GameSettingsWidgetExt};
use crate::widgets::player_settings::PlayerSettingsWidgetExt;
use crate::world::FromWorld;
use crate::{GamePauseState, GameSettings, GameState};

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Game)]
pub enum InPauseState {
    #[default]
    Default,
    PlayerSettings,
    GameSettings,
}

#[derive(Component)]
struct PauseWidget;

#[derive(Component)]
struct MainPauseItems;

#[derive(Component)]
struct PlayerSettingsContainer;

#[derive(Component)]
struct GameSettingsContainer;

pub struct PauseMenuPlugin;

impl Plugin for PauseMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<InPauseState>();

        app.add_systems(OnEnter(GameState::Game), setup);
        app.add_systems(
            OnExit(GameState::Game),
            |mut auto_apply_settings: ResMut<AutoApplySettings>| {
                *auto_apply_settings = AutoApplySettings(true)
            },
        );

        app.add_systems(OnEnter(GamePauseState::Paused), on_paused);
        app.add_systems(OnExit(GamePauseState::Paused), on_unpaused);

        app.add_systems(OnEnter(InPauseState::Default), on_enter_default_pause);
        app.add_systems(OnExit(InPauseState::Default), on_exit_default_pause);

        app.add_systems(
            OnEnter(InPauseState::PlayerSettings),
            on_enter_player_settings,
        );
        app.add_systems(
            OnExit(InPauseState::PlayerSettings),
            on_exit_player_settings,
        );

        app.add_systems(OnEnter(InPauseState::GameSettings), on_enter_game_settings);
        app.add_systems(OnExit(InPauseState::GameSettings), on_exit_game_settings);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut auto_apply_settings: ResMut<AutoApplySettings>,
) {
    *auto_apply_settings = AutoApplySettings(false);

    commands
        .ui_builder(UiRoot)
        .container(NodeBundle { ..default() }, |pause| {
            pause.named("Pause GUI");

            pause.insert(Visibility::Hidden);
            pause.insert(FocusPolicy::Block);

            pause.insert(PauseWidget);
            pause.insert(FromWorld);

            pause
                .style()
                .background_color(Color::srgba(0.0, 0.0, 0.0, 0.75))
                .width(Val::Percent(100.0))
                .height(Val::Percent(100.0))
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::Center);

            pause.column(|items| {
                items.insert(MainPauseItems);

                items
                    .style()
                    .height(Val::Percent(100.0))
                    .justify_content(JustifyContent::Center)
                    .row_gap(Val::Px(10.0))
                    .min_width(Val::Px(200.0))
                    .max_width(Val::Px(200.0));

                items
                    .spawn(
                        TextBundle::from_section(
                            "PAUSED",
                            TextStyle {
                                font: asset_server.load("fonts/nokiafc22.ttf"),
                                font_size: 120.0,
                                color: Color::WHITE,
                            },
                        )
                        .with_text_justify(JustifyText::Center),
                    )
                    .style()
                    .align_self(AlignSelf::Center);

                items.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(60.0),
                        ..default()
                    },
                    ..default()
                });

                items.button("Resume".into(), 40.0).observe(
                    |_: Trigger<ButtonPressed>, mut state: ResMut<NextState<GamePauseState>>| {
                        state.set(GamePauseState::Running);
                    },
                );

                items.button("Player Customization".into(), 24.0).observe(
                    |_: Trigger<ButtonPressed>, mut state: ResMut<NextState<InPauseState>>| {
                        state.set(InPauseState::PlayerSettings);
                    },
                );

                items.button("Settings".into(), 40.0).observe(
                    |_: Trigger<ButtonPressed>, mut state: ResMut<NextState<InPauseState>>| {
                        state.set(InPauseState::GameSettings);
                    },
                );

                items.button("Quit and Save".into(), 24.0).observe(
                    |_: Trigger<ButtonPressed>, mut ev: EventWriter<SaveAllChunks>| {
                        ev.send(SaveAllChunks);
                    },
                );
            });
        });
}

fn on_paused(mut pause_q: Query<&mut Visibility, With<PauseWidget>>) {
    if let Ok(mut vis) = pause_q.get_single_mut() {
        *vis = Visibility::Visible;
    }
}

fn on_unpaused(mut pause_q: Query<&mut Visibility, With<PauseWidget>>) {
    if let Ok(mut vis) = pause_q.get_single_mut() {
        *vis = Visibility::Hidden;
    }
}

fn on_enter_default_pause(mut pause_items_q: Query<&mut Style, With<MainPauseItems>>) {
    if let Ok(mut style) = pause_items_q.get_single_mut() {
        style.display = Display::DEFAULT;
    }
}

fn on_exit_default_pause(mut pause_items_q: Query<&mut Style, With<MainPauseItems>>) {
    if let Ok(mut style) = pause_items_q.get_single_mut() {
        style.display = Display::None;
    }
}

fn on_enter_player_settings(
    mut commands: Commands,
    pause_q: Query<Entity, With<PauseWidget>>,
    player_settings_res: Res<PlayerSettings>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(entity) = pause_q.get_single() {
        commands.ui_builder(entity).column(|column| {
            column.insert(PlayerSettingsContainer);

            column
                .style()
                .height(Val::Percent(100.0))
                .justify_content(JustifyContent::SpaceAround)
                .align_items(AlignItems::Center);

            column
                .spawn(
                    TextBundle::from_section(
                        "Player Customization",
                        TextStyle {
                            font: asset_server.load("fonts/nokiafc22.ttf"),
                            font_size: 100.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_justify(JustifyText::Center),
                )
                .style()
                .align_self(AlignSelf::Center);

            column
                .player_settings(&player_settings_res)
                .style()
                .min_width(Val::Px(500.0));

            column.button("< Go Back".into(), 40.0).observe(
                |_: Trigger<ButtonPressed>, mut state: ResMut<NextState<InPauseState>>| {
                    state.set(InPauseState::Default);
                },
            );
        });
    }
}

fn on_exit_player_settings(
    mut commands: Commands,
    query: Query<Entity, With<PlayerSettingsContainer>>,
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn on_enter_game_settings(
    mut commands: Commands,
    pause_q: Query<Entity, With<PauseWidget>>,
    settings_res: Res<GameSettings>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(entity) = pause_q.get_single() {
        commands.ui_builder(entity).column(|column| {
            column.insert(GameSettingsContainer);

            column
                .style()
                .height(Val::Percent(100.0))
                .justify_content(JustifyContent::SpaceAround)
                .align_items(AlignItems::Center);

            column
                .spawn(
                    TextBundle::from_section(
                        "Settings",
                        TextStyle {
                            font: asset_server.load("fonts/nokiafc22.ttf"),
                            font_size: 150.0,
                            color: Color::WHITE,
                        },
                    )
                    .with_text_justify(JustifyText::Center),
                )
                .style()
                .align_self(AlignSelf::Center);

            column.game_settings(&asset_server, &settings_res);

            column.row(|buttons| {
                buttons.style().justify_content(JustifyContent::SpaceAround);

                buttons.button("< Go Back".into(), 40.0).observe(
                    |_: Trigger<ButtonPressed>, mut state: ResMut<NextState<InPauseState>>| {
                        state.set(InPauseState::Default);
                    },
                );

                buttons.button("Apply".into(), 40.0).observe(
                    |_: Trigger<ButtonPressed>, mut ev: EventWriter<ApplyGameSettings>| {
                        ev.send(ApplyGameSettings);
                    },
                );
            });
        });
    }
}

fn on_exit_game_settings(
    mut commands: Commands,
    query: Query<Entity, With<GameSettingsContainer>>,
) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
