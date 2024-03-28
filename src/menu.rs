use std::{fs, io::ErrorKind};

use bevy::{a11y::{accesskit::{NodeBuilder, Role}, AccessibilityNode}, input::mouse::{MouseScrollUnit, MouseWheel}, prelude::*};

use crate::GameState;

#[derive(Component)]
struct MainMenu;

#[derive(Resource, Default)]
pub struct WorldInfo {
    pub path: Option<String>
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldInfo>();
        app.add_systems(OnEnter(GameState::Menu), (setup_menu, check_worlds_directory));
        app.add_systems(Update, mouse_scroll.run_if(in_state(GameState::Menu)));
        app.add_systems(OnExit(GameState::Menu), destroy_menu);
    }
}

fn check_worlds_directory() {
    let dir = fs::read_dir("worlds");

    if let Ok(files) = dir {
        for entry in files {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    println!("{:?}", entry.file_name());
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
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        MainMenu
    )).with_children(|parent| {
        parent.spawn(NodeBundle {
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
            }).with_children(|parent| {
                
                // Moving panel
                parent
                .spawn((
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
                    )).with_children(|parent| {
                        // List items
                        for i in 0..30 {
                            parent.spawn((
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
                                    format!("Item {i}"),
                                    TextStyle {
                                        font: asset_server.load("fonts/nokiafc22.ttf"),
                                        font_size: 24.0,
                                        ..default()
                                    }
                                ));
                            });
                        }
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
        
        #[derive(Component, Default)]
struct ScrollingList {
    position: f32,
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