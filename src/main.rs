mod utils;
mod chunk;
mod chunk_manager;
mod player;
mod menu;
mod world;

use std::{fs, io::ErrorKind};

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_xpbd_2d::prelude::*;
use menu::MenuPlugin;
use world::WorldPlugin;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum GameState {
    Menu,
    #[default]
    Game
}

fn main() {
    App::new()
        .init_state::<GameState>()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(PhysicsDebugPlugin::default())
        .add_plugins(PhysicsPlugins::default())
        .add_plugins(WorldPlugin)
        .add_plugins(MenuPlugin)

        .add_systems(Startup, (setup_worlds_folder, spawn_camera).chain())

        .run();
}

fn spawn_camera(
    mut commands: Commands
) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_worlds_folder() {
    let dir = fs::read_dir("worlds");        

    if let Err(err) = dir {
        if err.kind() == ErrorKind::NotFound {
            if let Err(e) = fs::create_dir("worlds") {
                error!("An error occurred when creating the worlds folder: {}", e);
            }
        } else {
            error!("An error occurred when checking for worlds directory: {}", err);
        }
    }
}