use crate::chunk::{self, PlaceMode, CHUNK_WIDTH, TILE_SIZE};
use crate::chunk_manager::{ChunkManagerPlugin, TryPlaceBlock};

use crate::player::{Player, PlayerPlugin};

use crate::{utils::*, GameState};

use bevy::{input::mouse::MouseWheel, prelude::*, sprite::SpriteBundle, window::PrimaryWindow};
use bevy_xpbd_2d::{prelude::*, SubstepSchedule, SubstepSet};

#[derive(Component)]
struct BlockCursor {
    layer: PlaceMode,
    block_position: Vec2,
    relative_position: Vec2,
    chunk_position: Vec2
}

#[derive(Component)]
struct FromWorld;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Gravity(Vec2::NEG_Y * (9.81 * TILE_SIZE as f32)))
           .add_plugins(ChunkManagerPlugin)
           .add_plugins(PlayerPlugin)
           .add_systems(OnEnter(GameState::Game), setup)
           .add_systems(Update, 
               (
                   update_cursor,
                   block_input,
                   switch_place_mode,
                   camera_zoom
               ).run_if(in_state(GameState::Game)))
           .add_systems(SubstepSchedule, camera_follow_player.in_set(SubstepSet::ApplyTranslation).run_if(in_state(GameState::Game)))
           ;
    }
}

fn setup(
    mut commands: Commands
) {
    commands.spawn((
        Name::new("Cursor"),
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(1.0, 1.0, 1.0, 0.5),
                custom_size: Some(Vec2::splat(TILE_SIZE as f32)),
                anchor: bevy::sprite::Anchor::BottomLeft,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        BlockCursor { 
            layer: PlaceMode::BLOCK,
            block_position: Vec2::ZERO,
            chunk_position: Vec2::ZERO,
            relative_position: Vec2::ZERO
        },
        FromWorld
    ));
}

fn block_input(
    cursor_q: Query<(&Transform, &BlockCursor)>,
    player_query: Query<&Transform, With<Player>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut try_place_block_ev: EventWriter<TryPlaceBlock>
) {
    if let Ok((cursor_transform, cursor)) = cursor_q.get_single() {
        if let Ok(player_transform) = player_query.get_single() {
            let player_to_block_pixel_pos = (player_transform.translation / TILE_SIZE as f32).floor() * TILE_SIZE as f32;
            if player_to_block_pixel_pos != cursor_transform.translation || cursor.layer == PlaceMode::WALL {
                if mouse_button_input.just_pressed(MouseButton::Right) {
                    try_place_block_ev.send(TryPlaceBlock { layer: cursor.layer, position: Vec2::new(cursor_transform.translation.x, cursor_transform.translation.y), id: 1 });
                }
            }
            if mouse_button_input.just_pressed(MouseButton::Left) {
                try_place_block_ev.send(TryPlaceBlock { layer: cursor.layer, position: Vec2::new(cursor_transform.translation.x, cursor_transform.translation.y), id: 0 });
            }
        }
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>
) {
    if let Ok(player_transform) = player_query.get_single() {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            camera_transform.translation.x = lerp(camera_transform.translation.x, player_transform.translation.x, 0.01);
            camera_transform.translation.y = lerp(camera_transform.translation.y, player_transform.translation.y, 0.01);
        }
    }
}

fn camera_zoom(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut mouse_scroll_event: EventReader<MouseWheel>
) {
    const CAMERA_MIN_ZOOM: f32 = 0.05;

    for ev in mouse_scroll_event.read() {
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                if vec3_a_bigger_than_b(camera_transform.scale, Vec3::splat(CAMERA_MIN_ZOOM)) {
                    if ev.y > 0.0 {
                        camera_transform.scale -= Vec3::splat(0.05);
                    }
                } else {
                    camera_transform.scale = Vec3::splat(CAMERA_MIN_ZOOM);
                }
    
                if ev.y < 0.0 {
                    camera_transform.scale += Vec3::splat(0.05);
                }
            }
        }
    }
}

fn update_cursor(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut cursor_q: Query<(&mut Transform, &mut BlockCursor, &mut Visibility)>,
    camera_q: Query<(&Camera, &GlobalTransform)>
) {
    let window = window_query.get_single().unwrap();
    let (mut cursor_transform, mut cursor, mut visibility) = cursor_q.single_mut();
    let (camera, camera_global_transform) = camera_q.single();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world_2d(camera_global_transform, cursor))
    {
        *visibility = Visibility::Visible;

        cursor.block_position = (world_position / chunk::TILE_SIZE as f32).floor();
        let g = cursor.block_position * TILE_SIZE as f32;
        cursor_transform.translation = Vec3::new(g.x, g.y, 0.0);

        cursor.chunk_position = (cursor.block_position / CHUNK_WIDTH as f32).floor();
        cursor.relative_position = (cursor.chunk_position * CHUNK_WIDTH as f32) - cursor.block_position;
    } else {
        *visibility = Visibility::Hidden;
    }
}

fn switch_place_mode(
    mut cursor_q: Query<&mut BlockCursor>,
    keyboard_input: Res<ButtonInput<KeyCode>>
) {
    let mut cursor = cursor_q.single_mut();
    if keyboard_input.just_pressed(KeyCode::Tab) {
        if cursor.layer == PlaceMode::WALL {
            cursor.layer = PlaceMode::BLOCK;
        } else if cursor.layer == PlaceMode::BLOCK {
            cursor.layer = PlaceMode::WALL;
        }

        println!("{:?}", cursor.layer);
    }
}