use crate::chunk::{self, BlockType, PlaceMode, CHUNK_WIDTH, TILE_SIZE};
use crate::chunk_manager::{ChunkManagerPlugin, LoadChunks, TryPlaceBlock, UnloadChunks};

use crate::player::{Player, PlayerPlugin};

use crate::{utils::*, GameState};

use bevy::{input::mouse::MouseWheel, prelude::*, sprite::SpriteBundle, window::PrimaryWindow};
use bevy_inspector_egui::quick::ResourceInspectorPlugin;
use bevy_xpbd_2d::{prelude::*, SubstepSchedule, SubstepSet};
use serde::{Deserialize, Serialize};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSystemSet {
    Chunk,
    ChunkManager,
    Player,
    World
}

#[derive(Clone, Copy, Debug, PartialEq, Reflect, Default, Serialize, Deserialize)]
pub enum WorldGenPreset {
    #[default]
    DEFAULT,
    FLAT,
    EMPTY
}

#[derive(Debug, Resource, Default, Reflect, Serialize, Deserialize)]
#[reflect(Resource)]
pub struct WorldInfo {
    pub display_name: String,
    pub name: String,
    pub preset: WorldGenPreset,
    pub last_player_pos: Vec2
}

#[derive(Component)]
struct BlockCursor {
    block_type: BlockType,
    layer: PlaceMode,
    block_position: IVec2,
    relative_position: UVec2,
    chunk_position: IVec2
}

#[derive(Component)]
struct CursorBlockIcon;

#[derive(Component)]
struct CursorPlaceModeIcon;

#[derive(Component)]
struct FromWorld;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldInfo { display_name: "".to_string(), name: "".to_string(), preset: WorldGenPreset::default(), last_player_pos: Vec2::ZERO });
        app.register_type::<WorldInfo>();
        app.add_plugins(ResourceInspectorPlugin::<WorldInfo>::default());
        
        app.insert_resource(Gravity(Vec2::NEG_Y * (9.81 * TILE_SIZE as f32)))
            .add_plugins(ChunkManagerPlugin)
            .add_plugins(PlayerPlugin)

            .configure_sets(OnEnter(GameState::Game), (
                GameSystemSet::Chunk,
                GameSystemSet::ChunkManager,
                GameSystemSet::Player,
                GameSystemSet::World
            ).chain().run_if(in_state(GameState::Game)))

            .configure_sets(Update, (
                GameSystemSet::ChunkManager,
                GameSystemSet::Chunk,
                GameSystemSet::World,
                GameSystemSet::Player
            ).chain().run_if(in_state(GameState::Game)))

            .add_systems(OnEnter(GameState::Game), (set_clear_color, setup).chain().in_set(GameSystemSet::World))
            .add_systems(Update, 
               (
                    update_cursor,
                    block_input,
                    switch_place_mode,
                    mouse_scroll_input,
                    force_reload_chunks
                ).in_set(GameSystemSet::World))
            .add_systems(SubstepSchedule, camera_follow_player.in_set(SubstepSet::ApplyTranslation).run_if(in_state(GameState::Game)))
            ;
    }
}

fn set_clear_color(
    mut camera_q: Query<&mut Camera>
) {
    let mut camera = camera_q.single_mut();
    camera.clear_color = ClearColorConfig::Custom(Color::rgb(0.48, 0.48, 0.67));
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>
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
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        BlockCursor {
            block_type: BlockType::GRASS,
            layer: PlaceMode::BLOCK,
            block_position: IVec2::ZERO,
            chunk_position: IVec2::ZERO,
            relative_position: UVec2::ZERO
        },
        FromWorld
    ));

    let layout = TextureAtlasLayout::from_grid(Vec2::splat(TILE_SIZE as f32), BlockType::SIZE as usize, 1, None, None);

    commands.spawn((
        Name::new("Block Indicator"),
        SpriteSheetBundle {
            texture: asset_server.load("textures/blocks.png"),
            atlas: TextureAtlas {
                layout: texture_atlas_layouts.add(layout),
                index: 0
            },
            sprite: Sprite {
                custom_size: Some(Vec2::splat(TILE_SIZE as f32 * 0.75)),
                anchor: bevy::sprite::Anchor::TopLeft,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
            visibility: Visibility::Hidden,
            ..default()
        },
        CursorBlockIcon,
        FromWorld
    )).with_children(|parent| {
        let new_layout = TextureAtlasLayout::from_grid(Vec2::splat(8.0), 2, 1, None, None);

        parent.spawn((SpriteSheetBundle {
                texture: asset_server.load("textures/place_modes.png"),
                atlas: TextureAtlas {
                    layout: texture_atlas_layouts.add(new_layout),
                    index: 1
                },
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(16.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(TILE_SIZE as f32 * 0.75, -(TILE_SIZE as f32 * 0.75), 4.0)),
                ..default()
            },
            CursorPlaceModeIcon
        ));
    });
}

fn block_input(
    cursor_q: Query<&BlockCursor>,
    player_query: Query<&Transform, With<Player>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut try_place_block_ev: EventWriter<TryPlaceBlock>
) {
    let cursor = cursor_q.single();
    let player_transform = player_query.single();

    let player_position = IVec2::new(
        (player_transform.translation.x / TILE_SIZE as f32).floor() as i32,
        (player_transform.translation.y / TILE_SIZE as f32).floor() as i32,
    );

    if player_position != cursor.block_position || cursor.layer == PlaceMode::WALL {
        if mouse_button_input.just_pressed(MouseButton::Right) {
            try_place_block_ev.send(TryPlaceBlock { layer: cursor.layer, position: cursor.block_position, id: cursor.block_type });
        }
    }
    if mouse_button_input.just_pressed(MouseButton::Left) {
        try_place_block_ev.send(TryPlaceBlock { layer: cursor.layer, position: cursor.block_position, id: BlockType::AIR });
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

fn mouse_scroll_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut unload_chunks_ev: EventWriter<UnloadChunks>,
    mut load_chunks_ev: EventWriter<LoadChunks>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
    mut cursor_query: Query<&mut BlockCursor>,
    mut cursor_block_icon_q: Query<&mut TextureAtlas, With<CursorBlockIcon>>,
    mut mouse_scroll_event: EventReader<MouseWheel>
) {
    const CAMERA_MIN_ZOOM: f32 = 0.05;
    const CAMERA_MAX_ZOOM: f32 = 2.0;

    for ev in mouse_scroll_event.read() {
        if keyboard_input.pressed(KeyCode::ControlLeft) {
            if let Ok(mut camera_transform) = camera_query.get_single_mut() {
                if vec3_a_bigger_than_b(camera_transform.scale, Vec3::splat(CAMERA_MIN_ZOOM)) {
                    if ev.y > 0.0 {
                        camera_transform.scale -= Vec3::splat(0.05);
                        //unload_chunks_ev.send(UnloadChunks { force: false });
                    }
                } else {
                    camera_transform.scale = Vec3::splat(CAMERA_MIN_ZOOM);
                }
                
                if vec3_a_smaller_than_b(camera_transform.scale, Vec3::splat(CAMERA_MAX_ZOOM)) {
                    if ev.y < 0.0 {
                        camera_transform.scale += Vec3::splat(0.05);
                        //unload_chunks_ev.send(UnloadChunks { force: false });
                    }
                } else {
                    camera_transform.scale = Vec3::splat(CAMERA_MAX_ZOOM);
                }
            }
        } else {
            let mut cursor = cursor_query.single_mut();
            if ev.y > 0.0 { // Scrolling up
                if cursor.block_type < BlockType::GLASS {
                    cursor.block_type = enum_iterator::next(&cursor.block_type).unwrap();
                }
            } else if ev.y < 0.0 { // Scrolling down
                if cursor.block_type > BlockType::GRASS {
                    cursor.block_type = enum_iterator::previous(&cursor.block_type).unwrap();
                }
            }

            let mut icon_tex_atlas = cursor_block_icon_q.single_mut();
            icon_tex_atlas.index = cursor.block_type as usize - 1;
        }
    }
}

fn update_cursor(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut cursor_q: Query<(&mut Transform, &mut BlockCursor, &mut Sprite, &mut Visibility)>,
    mut cursor_block_icon_q: Query<(&mut Transform, &mut Visibility), (With<CursorBlockIcon>, Without<BlockCursor>)>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    time: Res<Time>
) {
    if let Ok(window) = window_query.get_single() {
        let (mut cursor_transform, mut cursor, mut cursor_sprite, mut cursor_visibility) = cursor_q.single_mut();
        let (mut cursor_icon_transform, mut cursor_icon_visibility) = cursor_block_icon_q.single_mut();
        let (camera, camera_global_transform) = camera_q.single();
    
        cursor_sprite.color = Color::rgba(cursor_sprite.color.r(), cursor_sprite.color.g(), cursor_sprite.color.b(), (f32::sin(time.elapsed_seconds() * 4.0) / 4.0) + 0.25);
    
        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_global_transform, cursor))
        {
            *cursor_visibility = Visibility::Visible;
            *cursor_icon_visibility = Visibility::Visible;
    
            cursor_icon_transform.translation = Vec3::new(world_position.x, world_position.y, cursor_icon_transform.translation.z);
    
            cursor.block_position = IVec2::new((world_position.x as f32 / chunk::TILE_SIZE as f32).floor() as i32, (world_position.y as f32 / chunk::TILE_SIZE as f32).floor() as i32);
            let g = Vec2::new(cursor.block_position.x as f32, cursor.block_position.y as f32) * TILE_SIZE as f32;
            cursor_transform.translation = Vec3::new(g.x, g.y, cursor_transform.translation.z);
    
            cursor.chunk_position = IVec2::new((cursor.block_position.x as f32 * CHUNK_WIDTH as f32).floor() as i32, (cursor.block_position.y as f32 * CHUNK_WIDTH as f32).floor() as i32);
            let v = (cursor.chunk_position * CHUNK_WIDTH as i32) - cursor.block_position;
            cursor.relative_position = UVec2::new(v.x as u32, v.y as u32);
        } else {
            *cursor_visibility = Visibility::Hidden;
            *cursor_icon_visibility = Visibility::Hidden;
        }
    }
}

fn switch_place_mode(
    mut cursor_q: Query<&mut BlockCursor>,
    mut cursor_placemode_icon_q: Query<&mut TextureAtlas, With<CursorPlaceModeIcon>>,
    keyboard_input: Res<ButtonInput<KeyCode>>
) {
    if keyboard_input.just_pressed(KeyCode::Tab) {
        let mut cursor = cursor_q.single_mut();
        let mut place_mode_texture_atlas = cursor_placemode_icon_q.single_mut();

        if cursor.layer == PlaceMode::WALL {
            cursor.layer = PlaceMode::BLOCK;
        } else if cursor.layer == PlaceMode::BLOCK {
            cursor.layer = PlaceMode::WALL;
        }

        place_mode_texture_atlas.index = cursor.layer as usize;
    }
}

fn force_reload_chunks(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut unload_chunks_ev: EventWriter<UnloadChunks>
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        unload_chunks_ev.send(UnloadChunks {force: true});
    }
}