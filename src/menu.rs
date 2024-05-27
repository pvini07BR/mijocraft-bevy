use std::{fs, io::ErrorKind};

use bevy::{a11y::{accesskit::{NodeBuilder, Role}, AccessibilityNode}, input::mouse::{MouseScrollUnit, MouseWheel}, prelude::*};

use crate::GameState;

#[derive(Component)]
struct MainMenu;

#[derive(Component, Default)]
struct ScrollingList {
    position: f32,
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Menu), (setup_menu, check_worlds_directory).chain());
        app.add_systems(Update, mouse_scroll.run_if(in_state(GameState::Menu)));
        app.add_systems(OnExit(GameState::Menu), destroy_menu);
    }
}

fn check_worlds_directory(
    mut commands: Commands,
    scroll_list_q: Query<Entity, With<ScrollingList>>,
    asset_server: Res<AssetServer>
) {
    let scrollist_e = scroll_list_q.single();

    let dir = fs::read_dir("worlds");

    if let Ok(files) = dir {
        for entry in files {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    let world_name = entry.file_name().into_string().unwrap();

                    commands.spawn((
                        Name::new(format!("World Entry: \"{}\"", world_name)),
                        ButtonBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                margin: UiRect::all(Val::Px(4.0)),
                                padding: UiRect::all(Val::Px(4.0)),
                                border: UiRect::all(Val::Px(4.0)),
                                justify_content: JustifyContent::Start,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            border_color: BorderColor(Color::WHITE),
                            background_color: BackgroundColor(Color::BLACK),
                            ..default()
                        },
                        Label,
                        AccessibilityNode(NodeBuilder::new(Role::ListItem)),
                    )).with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            world_name,
                            TextStyle {
                                font: asset_server.load("fonts/nokiafc22.ttf"),
                                font_size: 24.0,
                                ..default()
                            }
                        ));
                    }).set_parent(scrollist_e);
                }
            }
        }

    } else if let Err(err) = dir {
        if err.kind() == ErrorKind::NotFound {
            if let Err(e) = fs::create_dir("worlds") {
                println!("An error occurred when creating the worlds folder: {}", e);
            }
        } else {
            println!("An error occurred when checking for worlds directory: {}", err);
        }
    }
}

fn setup_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    commands.spawn((
        Name::new("UI Container"),
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        MainMenu
    )).with_children(|parent| {
        parent.spawn((
            Name::new("World Picker"),
            NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_self: AlignSelf::Center,
                    width: Val::Percent(50.0),
                    height: Val::Percent(50.),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                background_color: Color::rgb(0.10, 0.10, 0.10).into(),
                ..default()
            })).with_children(|world_picker_parent| {
                // Moving panel
                world_picker_parent.spawn((
                    Name::new("Scrolling Panel"),
                    NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: UiRect {
                                    left: Val::Px(10.0),
                                    right: Val::Px(10.0),
                                    top: Val::Px(6.0),
                                    bottom: Val::Px(6.0)
                                },
                                ..default()
                            },
                            ..default()
                        },
                        ScrollingList::default(),
                        AccessibilityNode(NodeBuilder::new(Role::List)),
                    ));
                });
        parent.spawn((
            Name::new("World Picker Buttons"),
            NodeBundle {
                style: Style {
                    width: Val::Percent(50.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..default()
                },
                ..default()
            }
        )).with_children(|w_p_buttons_parent| {
            w_p_buttons_parent.spawn((
                Name::new("Create World Button"),
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        margin: UiRect::all(Val::Px(4.0)),
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(4.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::WHITE),
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                },
            )).with_children(|button_parent| {
                button_parent.spawn(TextBundle::from_section(
                    "Create World",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        ..default()
                    }
                ));
            });

            w_p_buttons_parent.spawn((
                Name::new("Another Button"),
                ButtonBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        margin: UiRect::all(Val::Px(4.0)),
                        padding: UiRect::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(4.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    border_color: BorderColor(Color::WHITE),
                    background_color: BackgroundColor(Color::BLACK),
                    ..default()
                },
            )).with_children(|button_parent| {
                button_parent.spawn(TextBundle::from_section(
                    "Another Button",
                    TextStyle {
                        font: asset_server.load("fonts/nokiafc22.ttf"),
                        font_size: 24.0,
                        ..default()
                    }
                ));
            });
        });
    });
}
        
fn destroy_menu(
    mut commands: Commands,
    entity_query: Query<Entity, With<MainMenu>>
) {
    for entity in entity_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn mouse_scroll(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut query_list: Query<(&mut ScrollingList, &mut Style, &Parent, &Node)>,
    query_node: Query<&Node>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        for (mut scrolling_list, mut style, parent, list_node) in &mut query_list {
            let items_height = list_node.size().y;
            let container_height = query_node.get(parent.get()).unwrap().size().y;

            let max_scroll = (items_height - container_height).max(0.);

            let dy = match mouse_wheel_event.unit {
                MouseScrollUnit::Line => mouse_wheel_event.y * 20.,
                MouseScrollUnit::Pixel => mouse_wheel_event.y,
            };

            scrolling_list.position += dy;
            scrolling_list.position = scrolling_list.position.clamp(-max_scroll, 0.);
            style.top = Val::Px(scrolling_list.position);
        }
    }
}