use std::fs;

use bevy::{app::AppExit, prelude::*, ui::FocusPolicy};
use bevy_simple_text_input::{TextInputBundle, TextInputInactive, TextInputPlugin, TextInputSettings, TextInputValue};
use filenamify::filenamify;
use sickle_ui::{prelude::*, SickleUiPlugin};
use crate::{chunk_manager::JustCreatedWorld, world::{WorldGenPreset, WorldInfo}, GameState};

#[derive(Component)]
struct PlayButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct PlaySelectedWorldButton;

#[derive(Component)]
struct CreateWorldButton;

#[derive(Component)]
struct OpenWorldsFolderButton;

#[derive(Component)]
struct RefreshWorldListButton;

#[derive(Component)]
struct CreateWorldAndPlayButton;

#[derive(Component)]
struct ExitWorldCreationButton;

#[derive(Component)]
struct ExitWorldSelectionScreenButton;

#[derive(Component)]
struct WorldCreationNameTextInput;

#[derive(Component)]
struct WorldGenPresetDropdown;

#[derive(Component)]
struct WorldListEntry {
    world_info: WorldInfo
}

// This is for the entities composing the default main menu in InMenuState enum
#[derive(Component)]
struct DefaultMenu;

// This is for the entities composing the world screen menu in InMenuState enum
#[derive(Component)]
struct WorldScreenMenu;

// This is for the entities composing the settings menu in InMenuState enum
#[derive(Component)]
struct SettingsMenu;

#[derive(Component)]
struct WorldCreation;

#[derive(Component)]
struct WorldListScroll;

#[derive(Event)]
struct RefreshWorldList;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum InMenuState {
    #[default]
    Default,
    WorldScreen,
    SettingsMenu
}

#[derive(Resource, Default)]
struct WorldListEntryEID(Option<Entity>);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SickleUiPlugin);
        app.add_plugins(TextInputPlugin);

        app.add_event::<RefreshWorldList>();

        app.init_state::<InMenuState>();
        app.insert_resource(WorldListEntryEID(None));
        app.add_systems(OnEnter(GameState::Menu), setup_menu);
        app.add_systems(Update, (
            button_system,
            on_play_button_pressed,
            on_exit_pressed,
            world_list_entry_system,
            on_exit_world_selection_pressed,
            on_play_selected_world_pressed,
            on_create_world_pressed,
            on_open_worlds_folder_pressed,
            on_refresh_world_list,
            on_refresh_world_list_pressed,
            on_exit_world_creation_pressed,
            on_create_world_and_play_pressed,
        ).run_if(in_state(GameState::Menu)));
        app.add_systems(OnExit(GameState::Menu), destroy_menu);

        app.add_systems(OnEnter(InMenuState::Default), setup_default_menu.run_if(in_state(GameState::Menu)));
        app.add_systems(OnExit(InMenuState::Default), destroy_default_menu.run_if(in_state(GameState::Menu)));

        app.add_systems(OnEnter(InMenuState::WorldScreen), setup_world_screen_menu.run_if(in_state(GameState::Menu)));
        app.add_systems(OnExit(InMenuState::WorldScreen), destroy_world_screen_menu.run_if(in_state(GameState::Menu)));
    }
}

fn setup_menu(
    mut menu_state: ResMut<NextState<InMenuState>>
) {
    menu_state.set(InMenuState::Default);
}

fn destroy_menu(
    mut commands: Commands,
    menu_entities_query: Query<Entity, Or<(With<DefaultMenu>, With<WorldScreenMenu>, With<SettingsMenu>)>>
) {
    for entity in menu_entities_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_default_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    let font = asset_server.load("fonts/nokiafc22.ttf");
    let logo_text_style = TextStyle {
        font: font.clone(),
        font_size: 150.0,
        color: Color::WHITE,
    };

    let button_style = Style {
        width: Val::Px(200.0),
        height: Val::Px(65.0),
        border: UiRect::all(Val::Px(5.0)),
        margin: UiRect::all(Val::Px(5.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };

    commands.ui_builder(UiRoot).row(|row| {
        row.named("Default Main Menu Screen");
        row.style().height(Val::Percent(100.0));
        row.insert(DefaultMenu);

        row.spawn((
            Name::new("Version Text"),
            TextBundle::from_section("Alpha v0.1.0", TextStyle {
                font: font.clone(),
                font_size: 24.0,
                color: Color::WHITE,
            }).with_style(Style {
                align_self: AlignSelf::FlexEnd,
                min_width: Val::Px(221.0),
                ..default()
            }),
        ));

        row.column(|column| {
            column.style()
                .width(Val::Percent(100.0))
                .justify_content(JustifyContent::Center)
                .align_items(AlignItems::Center)
                .row_gap(Val::Px(100.0));
            column.insert(Name::new("Menu"));
    
            column.spawn((
                Name::from("Logo"),
                TextBundle::from_section("mijocraft", logo_text_style).with_text_justify(JustifyText::Center),
            ));
    
            column.container(
                NodeBundle {
                    ..default()
                }
            , |container| {
                container.style().flex_direction(FlexDirection::Column);
    
                container.spawn((
                    Name::new("Play Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    PlayButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                container.spawn((
                    Name::new("Player Customization Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    }
                )).entity_commands().with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                        "Player Customization",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });
        
                container.spawn((
                    Name::new("Settings Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    }
                )).entity_commands().with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                        "Settings",
                        TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                container.spawn((
                    Name::new("Quit Button"),
                    ButtonBundle {
                        style: button_style,
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    QuitButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(
                        TextBundle::from_section(
                        "Quit",
                        TextStyle {
                            font: font.clone(),
                            font_size: 40.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });
            });
        });

        row.spawn((
            Name::new("Credits Text"),
            TextBundle::from_section("Made by pvini07BR", TextStyle {
                font: font,
                font_size: 24.0,
                color: Color::WHITE,
            }).with_style(Style {
                align_self: AlignSelf::FlexEnd,
                min_width: Val::Px(221.0),
                ..default()
            })
        ));
    });
}

fn destroy_default_menu(
    mut commands: Commands,
    default_menu_q: Query<Entity, With<DefaultMenu>>
) {
    for entity in default_menu_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_world_screen_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut refresh_world_list_ev: EventWriter<RefreshWorldList>
) {
    let font = asset_server.load("fonts/nokiafc22.ttf");

    let button_style = Style {
        width: Val::Percent(100.0),
        //height: Val::Px(40.0),
        border: UiRect::all(Val::Px(5.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: UiRect::horizontal(Val::Px(5.0)),
        ..default()
    };

    commands.ui_builder(UiRoot).row(|row| {
        row.insert(WorldScreenMenu);
        row.named("World Screen Menu");
        row.style()
            .height(Val::Percent(100.0))
            .justify_content(JustifyContent::Center)
            .align_items(AlignItems::Center);

        row.container(NodeBundle { ..default() }, |world_sel| {
            world_sel.named("World Selection");
            world_sel.style()
                .flex_direction(FlexDirection::Column)
                .background_color(Color::BLACK)
                .border_color(Color::WHITE)
                .padding(UiRect::all(Val::Px(5.0)))
                .border(UiRect::all(Val::Px(5.0)))
                .min_height(Val::Px(550.0))
                .row_gap(Val::Px(5.0))
            ;

            world_sel.scroll_view(None, |world_scroll_view| {
                world_scroll_view.style()
                    .background_color(Color::hex("#2b2c2f").unwrap())
                ;
                world_scroll_view.insert(WorldListScroll);
            });

            world_sel.container(NodeBundle { ..default() }, |button_container| {
                button_container.named("Buttons").style()
                    .flex_direction(FlexDirection::Row)
                    .column_gap(Val::Px(5.0))
                    .height(Val::Px(100.0))
                ;

                button_container.spawn((
                    Name::new("Exit World Selection Screen Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    ExitWorldSelectionScreenButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "< Go Back",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                button_container.spawn((
                    Name::new("Play World Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    PlaySelectedWorldButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play Selected World",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                button_container.spawn((
                    Name::new("Create World Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    CreateWorldButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Create World",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                button_container.spawn((
                    Name::new("Open Worlds Folder Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    OpenWorldsFolderButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Open Worlds Folder",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });

                button_container.spawn((
                    Name::new("Refresh Button"),
                    ButtonBundle {
                        style: button_style.clone(),
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    RefreshWorldListButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Refresh",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ).with_text_justify(JustifyText::Center));
                });
            });
        });

        row.container(NodeBundle { ..default() }, |fade| {
            fade.named("Fade");
            fade.style()
                .position_type(PositionType::Absolute)
                .background_color(Color::rgba(0.0, 0.0, 0.0, 0.75))
                .width(Val::Percent(100.0))
                .height(Val::Percent(100.0));
            fade.insert(FocusPolicy::Block);
            fade.insert(Visibility::Hidden);
            fade.insert(WorldCreation);
        });

        row.container(NodeBundle { ..default() }, |world_creation| {
            world_creation.named("World Creation");
            world_creation.insert(Visibility::Hidden);
            world_creation.insert(WorldCreation);
            world_creation.style()
                .position_type(PositionType::Absolute)
                .flex_direction(FlexDirection::Column)
                .background_color(Color::BLACK)
                .border_color(Color::WHITE)
                .padding(UiRect::all(Val::Px(15.0)))
                .border(UiRect::all(Val::Px(5.0)))
                .row_gap(Val::Px(5.0));

            world_creation.column(|entries| {
                entries.style().row_gap(Val::Px(15.0));

                entries.spawn((
                    Name::new("World Creation: Go Back"),
                    ButtonBundle {
                        style: Style {
                            height: Val::Px(40.0),
                            border: UiRect::all(Val::Px(5.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(5.0)),
                            align_self: AlignSelf::Start,
                            ..default()
                        },
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    ExitWorldCreationButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "< Go Back",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });

                entries.row(|world_name_entry| {
                    world_name_entry.style().column_gap(Val::Px(5.0)).justify_content(JustifyContent::SpaceBetween);

                    world_name_entry.spawn(TextBundle::from_section(
                        "World Name: ",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    )).style().width(Val::Percent(100.0));

                    world_name_entry.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                border: UiRect::all(Val::Px(5.0)),
                                padding: UiRect::all(Val::Px(5.0)),
                                max_width: Val::Px(200.0),
                                ..default()
                            },
                            border_color: Color::WHITE.into(),
                            background_color: Color::BLACK.into(),
                            ..default()
                        },
                        TextInputBundle::default()
                            .with_text_style(TextStyle {
                                font: font.clone(),
                                font_size: 24.0,
                                color: Color::WHITE,
                                ..default()
                            }).with_settings(TextInputSettings { retain_on_submit: true, ..default() }).with_inactive(true),
                        WorldCreationNameTextInput
                    ));
                });

                entries.row(|world_gen_preset_entry| {
                    world_gen_preset_entry.style().column_gap(Val::Px(5.0)).justify_content(JustifyContent::SpaceBetween);

                    world_gen_preset_entry.spawn(TextBundle::from_section(
                        "World Generation Preset: ",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));

                    world_gen_preset_entry.dropdown(vec!["Default", "Flat", "Empty"], 0).insert(WorldGenPresetDropdown);
                });

                entries.spawn((
                    Name::new("World Creation: Create World and Play"),
                    ButtonBundle {
                        style: button_style,
                        border_color: BorderColor(Color::WHITE),
                        background_color: BackgroundColor(Color::BLACK),
                        ..default()
                    },
                    CreateWorldAndPlayButton
                )).entity_commands().with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Create World and Play",
                        TextStyle {
                            font: font.clone(),
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    ));
                });
            });
        });

        refresh_world_list_ev.send(RefreshWorldList);
    });
}

fn destroy_world_screen_menu(
    mut commands: Commands,
    default_menu_q: Query<Entity, With<WorldScreenMenu>>
) {
    for entity in default_menu_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn on_play_button_pressed(
    inter_q: Query<&Interaction, With<PlayButton>>,
    mut state: ResMut<NextState<InMenuState>>,
    input: Res<ButtonInput<MouseButton>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                state.set(InMenuState::WorldScreen);
            }
        }
    }
}

fn on_exit_pressed(
    inter_q: Query<&Interaction, With<QuitButton>>,
    input: Res<ButtonInput<MouseButton>>,
    mut exit: EventWriter<AppExit>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                exit.send(AppExit);
            }
        }
    }
}

fn on_exit_world_selection_pressed(
    inter_q: Query<&Interaction, With<ExitWorldSelectionScreenButton>>,
    mut state: ResMut<NextState<InMenuState>>,
    input: Res<ButtonInput<MouseButton>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                state.set(InMenuState::Default);
            }
        }
    }
}

fn on_play_selected_world_pressed(
    inter_q: Query<&Interaction, With<PlaySelectedWorldButton>>,
    input: Res<ButtonInput<MouseButton>>,
    world_entry_q: Query<&WorldListEntry>,
    index_res: Res<WorldListEntryEID>,
    mut world_info_res: ResMut<WorldInfo>,
    mut state: ResMut<NextState<GameState>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                match index_res.0 {
                    Some(e) => {
                        *world_info_res = world_entry_q.get(e).unwrap().world_info.clone();
                        state.set(GameState::Game);
                    }
                    None => {}
                }
            }
        }
    }
}

fn on_create_world_pressed(
    inter_q: Query<&Interaction, With<CreateWorldButton>>,
    mut world_creation_q: Query<&mut Visibility, With<WorldCreation>>,
    mut text_input_q: Query<&mut TextInputInactive, With<WorldCreationNameTextInput>>,
    input: Res<ButtonInput<MouseButton>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                for mut visibility in world_creation_q.iter_mut() {
                    *visibility = Visibility::Visible;
                }

                let mut text_input_inactive = text_input_q.get_single_mut().unwrap();
                *text_input_inactive = TextInputInactive(false);
            }
        }
    }
}

fn on_open_worlds_folder_pressed(
    inter_q: Query<&Interaction, With<OpenWorldsFolderButton>>,
    input: Res<ButtonInput<MouseButton>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                if let Err(e) = opener::open("./worlds") {
                    error!("Failed opening worlds folder: {}", e);
                }
            }
        }
    }
}

fn on_refresh_world_list_pressed(
    inter_q: Query<&Interaction, With<RefreshWorldListButton>>,
    input: Res<ButtonInput<MouseButton>>,
    mut ev: EventWriter<RefreshWorldList>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                ev.send(RefreshWorldList);
            }
        }
    }
}

fn on_create_world_and_play_pressed(
    inter_q: Query<&Interaction, With<CreateWorldAndPlayButton>>,
    mut next_state: ResMut<NextState<GameState>>,
    text_input_query: Query<&TextInputValue, With<WorldCreationNameTextInput>>,
    world_preset_dropdown_q: Query<&Dropdown, With<WorldGenPresetDropdown>>,
    input: Res<ButtonInput<MouseButton>>,
    mut world_info_res: ResMut<WorldInfo>,
    mut first_time: ResMut<JustCreatedWorld>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                let preset = match world_preset_dropdown_q.single().value().unwrap() {
                    0 => WorldGenPreset::DEFAULT,
                    1 => WorldGenPreset::FLAT,
                    2 => WorldGenPreset::EMPTY,
                    _ => WorldGenPreset::DEFAULT
                };

                let text_input = text_input_query.single();
                if text_input.0.trim().is_empty() {
                    error!("You cannot enter in a empty world name!");
                    return;
                }

                let world_name = filenamify(text_input.0.clone()).to_lowercase().replace(" ", "_");
                info!("World will be saved as '{}'.", world_name.clone());

                let info = WorldInfo {
                    display_name: text_input.0.clone(),
                    name: world_name.clone(),
                    preset,
                    last_player_pos: Vec2::ZERO
                };

                *first_time = JustCreatedWorld(true);

                if let Err(e) = fs::create_dir(format!("worlds/{}", world_name.clone())) {
                    error!("Failed creating world directory for '{}': {}", world_name, e);
                } else {
                    if let Err(e) = fs::create_dir(format!("worlds/{}/chunks", world_name.clone())) {
                        error!("Failed creating the chunks directory for '{}': {}", world_name, e);
                    }
                    
                    match toml::to_string(&info) {
                        Ok(str) => {
                            if let Err(e) = fs::write(format!("worlds/{}/world.toml", world_name.clone()), str) {
                                error!("Could not write the world information into a file: {}", e);
                            }
                        },
                        Err(e) => error!("Could not serialize world information into a string: {}", e),
                    }
                }

                *world_info_res = info;

                next_state.set(GameState::Game);
            }
        }
    }
}

fn on_exit_world_creation_pressed(
    inter_q: Query<&Interaction, With<ExitWorldCreationButton>>,
    mut world_creation_q: Query<&mut Visibility, With<WorldCreation>>,
    mut text_input_q: Query<&mut TextInputInactive, With<WorldCreationNameTextInput>>,
    input: Res<ButtonInput<MouseButton>>
) {
    if let Ok(inter) = inter_q.get_single() {
        if input.just_released(MouseButton::Left) {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                for mut visibility in world_creation_q.iter_mut() {
                    *visibility = Visibility::Hidden;
                }

                let mut text_input_inactive = text_input_q.get_single_mut().unwrap();
                *text_input_inactive = TextInputInactive(true);
            }
        }
    }
}

fn on_refresh_world_list(
    mut commands: Commands,
    mut ev: EventReader<RefreshWorldList>,
    world_list_q: Query<Entity, With<WorldListScroll>>,
    mut entry_eid_res: ResMut<WorldListEntryEID>,
    asset_server: Res<AssetServer>
) {
    for _ in ev.read() {
        let mut world_scroll_view = commands.ui_builder(world_list_q.single());

        world_scroll_view.entity_commands().despawn_descendants();
        entry_eid_res.0 = None;

        let font = asset_server.load("fonts/nokiafc22.ttf");

        let mut found_worlds: bool = false;
    
        match fs::read_dir("worlds") {
            Ok(dirs) => {
                for dir in dirs {
                    match dir {
                        Ok(entry) => {
                            if entry.path().is_dir() {
                                found_worlds = true;

                                let world_name = entry.file_name().into_string().unwrap();
                                
                                world_scroll_view.container(NodeBundle {..default()}, |world_thing| {
                                    match fs::read_to_string(format!("{}/world.toml", entry.path().display())) {
                                        Ok(str) => {
                                            match toml::from_str::<WorldInfo>(&str) {
                                                Ok(world_info) => {

                                                    world_thing.spawn((
                                                        Name::new(format!("World entry: {}", world_name)),
                                                        ButtonBundle {
                                                            style: Style {
                                                                flex_direction: FlexDirection::Column,
                                                                border: UiRect::all(Val::Px(2.0)),
                                                                margin: UiRect::all(Val::Px(5.0)),
                                                                padding: UiRect::all(Val::Px(5.0)),
                                                                width: Val::Percent(100.0),
                                                                ..default()
                                                            },
                                                            ..default()
                                                        },
                                                        WorldListEntry {world_info: world_info.clone()}
                                                    )).style().background_color(Color::rgba(0.0, 0.0, 0.0, 0.0)).entity_commands().with_children(|parent| {
                                                        parent.spawn(
                                                            TextBundle::from_section(world_info.display_name, TextStyle {
                                                                font: font.clone(),
                                                                font_size: 30.0,
                                                                color: Color::WHITE,
                                                        }));
    
                                                        let world_preset_string = match world_info.preset {
                                                            WorldGenPreset::DEFAULT => "Default",
                                                            WorldGenPreset::FLAT => "Flat",
                                                            WorldGenPreset::EMPTY => "Empty"
                                                        };
    
                                                        parent.spawn(
                                                            TextBundle::from_section(format!("Type: {}", world_preset_string), TextStyle {
                                                                font: font.clone(),
                                                                font_size: 20.0,
                                                                color: Color::WHITE,
                                                        }));
                                                        
                                                        parent.spawn(
                                                            TextBundle::from_section(format!("Saved as: {}", world_name), TextStyle {
                                                                font: font.clone(),
                                                                font_size: 16.0,
                                                                color: Color::GRAY,
                                                        }));
                                                    });
                                                },
                                                Err(e) => error!("Failed to parse world.toml for world '{}': {}", world_name, e)
                                            }
                                        },
                                        Err(e) => {
                                            error!("Failed to read world.toml from '{}' world: {}", world_name, e);
                                        }
                                    }
                                });
                            }
                        },
                        Err(e) => error!("An error ocurred when reading a world directory entry: {}", e)
                    }
                }
            },
            Err(e) => error!("An error occurred when checking for worlds directory: {}", e)
        }
    
        if !found_worlds {
            world_scroll_view.spawn(
                TextBundle::from_section("No worlds were found.",
                TextStyle {
                    font: font.clone(),
                    font_size: 24.0,
                    color: Color::GRAY,
                }).with_text_justify(JustifyText::Center)
            ).style().height(Val::Percent(100.0));
        }
    }
}

fn world_list_entry_system(
    mut entry_inter_q: Query<(Entity, &Interaction), With<WorldListEntry>>,
    mut entry_border_q: Query<&mut BorderColor, With<WorldListEntry>>,
    entry_worldinfo_q: Query<&WorldListEntry>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut entry_eid: ResMut<WorldListEntryEID>,
    mut next_state: ResMut<NextState<GameState>>,
    mut world_info_res: ResMut<WorldInfo>
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        let mut entry_eid_d: Entity = Entity::PLACEHOLDER;

        for (entity, inter) in entry_inter_q.iter_mut() {
            if *inter == Interaction::Hovered || *inter == Interaction::Pressed {
                entry_eid_d = entity;
                break;
            }
        }

        if entry_eid_d != Entity::PLACEHOLDER {
            if entry_eid.0 != Some(entry_eid_d) {
                for mut bc in entry_border_q.iter_mut() {
                    *bc = BorderColor(Color::rgba(0., 0., 0., 0.));
                }

                let mut border_color = entry_border_q.get_mut(entry_eid_d).unwrap();
                *border_color = Color::GRAY.into();
                entry_eid.0 = Some(entry_eid_d);
            } else {
                *world_info_res = entry_worldinfo_q.get(entry_eid_d).unwrap().world_info.clone();
                next_state.set(GameState::Game);
            }
        }
    }
}

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children
        ),
        (Changed<Interaction>, With<Button>, Without<WorldListEntry>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        if let Ok(mut text) = text_query.get_mut(children[0]) {
            match *interaction {
                Interaction::None => {
                    *color = BackgroundColor(Color::BLACK);
                    border_color.0 = Color::WHITE;
                    text.sections[0].style.color = Color::WHITE;
                },
                Interaction::Hovered => {
                    *color = BackgroundColor(Color::GRAY);
                    border_color.0 = Color::WHITE;
                    text.sections[0].style.color = Color::WHITE;
                }
                Interaction::Pressed => {
                    *color = BackgroundColor(Color::WHITE);
                    border_color.0 = Color::BLACK;
                    text.sections[0].style.color = Color::BLACK;
                },
            }
        }
    }
}