mod chunk;
mod chunk_manager;
mod menu;
mod pause_menu;
mod player;
mod utils;
mod widgets;
mod world;

use bevy::prelude::*;
use bevy_xpbd_2d::prelude::*;
use menu::MenuPlugin;
use sickle_ui::prelude::ThemeData;
use std::{fs, io::ErrorKind};
use world::WorldPlugin;

#[derive(States, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub enum GameState {
    #[default]
    Menu,
    Game,
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Game)]
pub enum GamePauseState {
    #[default]
    Running,
    Paused,
}

#[derive(Resource, Reflect, Default, Clone)]
#[reflect(Resource)]
pub struct GameSettings {
    pub wall_ambient_occlusion: bool,
    pub smooth_lighting: bool,
    pub wall_darkness: f32,
}

#[derive(Component)]
pub struct MainCamera;

fn main() {
    App::new()
        .insert_resource(GameSettings {
            smooth_lighting: true,
            wall_ambient_occlusion: true,
            wall_darkness: 0.5,
        })
        .register_type::<GameSettings>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        //.add_plugins(WorldInspectorPlugin::new())
        //.add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(WorldPlugin)
        .add_plugins(MenuPlugin)
        .init_state::<GameState>()
        .add_sub_state::<GamePauseState>()
        .add_systems(
            Startup,
            (setup_theme, setup_worlds_folder, spawn_camera).chain(),
        )
        .run();
}

fn setup_theme(mut theme_data: ResMut<ThemeData>) {
    theme_data.text.body.medium.font.regular = "fonts/nokiafc22.ttf".to_string();
    theme_data.text.body.medium.size = 24.0;

    theme_data.colors.core_colors.primary = Color::srgb(0.0, 0.0, 1.0).into()
}

fn setup_worlds_folder() {
    let dir = fs::read_dir("worlds");

    if let Err(err) = dir {
        if err.kind() == ErrorKind::NotFound {
            warn!("Could not find the worlds directory. Creating a new one...");
            warn!("If you already had a worlds directory, please delete the newly created worlds directory");
            warn!(
                "and check if the game is running on the same directory as the worlds directory."
            );

            if let Err(e) = fs::create_dir("worlds") {
                error!(
                    "An error occurred when creating the worlds directory: {}",
                    e
                );
            }
        } else {
            error!(
                "An error occurred when checking for worlds directory: {}",
                err
            );
        }
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera));
}
