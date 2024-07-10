use crate::chunk::{self, BlockType, PlaceMode, CHUNK_AREA, CHUNK_WIDTH, TILE_SIZE};
use crate::chunk_manager::{ChunkManagerPlugin, FinishedSavingChunks, TryPlaceBlock, UnloadChunks};

use crate::pause_menu::{InPauseState, PauseMenuPlugin};
use crate::player::{Player, PlayerPlugin};

use crate::{utils::*, GamePauseState, GameState, MainCamera};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::sprite::MaterialMesh2dBundle;
use bevy::{input::mouse::MouseWheel, prelude::*, sprite::SpriteBundle, window::PrimaryWindow};
use bevy_xpbd_2d::{prelude::*, SubstepSchedule, SubstepSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Reflect, Default, Serialize, Deserialize)]
pub enum WorldGenPreset {
    #[default]
    DEFAULT,
    FLAT,
    EMPTY,
}

#[derive(Debug, Resource, Default, Reflect, Serialize, Deserialize, Clone)]
#[reflect(Resource)]
pub struct WorldInfo {
    pub display_name: String,
    pub name: String,
    pub preset: WorldGenPreset,
    pub player_position: Option<Vec2>, // THIS IS IN BLOCK UNITS!!!
    pub is_flying: bool
}

#[derive(Component)]
struct BlockCursor {
    block_type: BlockType,
    layer: PlaceMode,
    block_position: IVec2,
    relative_position: UVec2,
    chunk_position: IVec2,
}

#[derive(Component)]
struct CursorBlockIcon;

#[derive(Component)]
struct CursorPlaceModeIcon;

#[derive(Component)]
struct SkyBackground;

#[derive(Component)]
pub struct FromWorld;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldInfo {
            display_name: "".to_string(),
            name: "".to_string(),
            preset: WorldGenPreset::default(),
            player_position: None,
            is_flying: false
        })
        .insert_resource(Gravity(Vec2::NEG_Y * (9.81 * TILE_SIZE as f32)))
        .register_type::<WorldInfo>()
        .add_plugins((ChunkManagerPlugin, PlayerPlugin, PauseMenuPlugin))
        .add_systems(
            OnEnter(GameState::Game),
            (config_camera, setup, setup_sky_bg)
                .chain()
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(
            Update,
            (
                (switch_place_mode, mouse_scroll_input, force_reload_chunks)
                    .run_if(in_state(GamePauseState::Running)),
                // The pause input system will be ran in both running and paused states
                pause_input,
                on_finished_saving_chunks,
            )
                .run_if(in_state(GameState::Game)),
        )
        .add_systems(
            SubstepSchedule,
            (
                camera_follow_player,
                update_sky_bg,
                update_cursor,
                block_input,
            )
                .chain()
                .in_set(SubstepSet::ApplyTranslation)
                .run_if(in_state(GameState::Game))
                .run_if(in_state(GamePauseState::Running)),
        )
        .add_systems(OnExit(GameState::Game), destroy_game)
        .add_systems(OnEnter(GamePauseState::Paused), on_game_paused)
        .add_systems(OnExit(GamePauseState::Paused), on_game_unpaused);
    }
}

fn config_camera(
    mut camera_q: Query<(&mut Camera, &mut Transform), With<MainCamera>>,
    world_info: Res<WorldInfo>,
) {
    let (mut camera, mut camera_transform) = camera_q.single_mut();
    camera.clear_color = ClearColorConfig::Custom(Color::BLACK);

    if let Some(pos) = world_info.player_position {
        camera_transform.translation.x = pos.x as f32 * TILE_SIZE as f32;
        camera_transform.translation.y = pos.y as f32 * TILE_SIZE as f32;
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn((
        Name::new("Cursor"),
        SpriteBundle {
            sprite: Sprite {
                color: Color::srgba(1.0, 1.0, 1.0, 0.5),
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
            relative_position: UVec2::ZERO,
        },
        FromWorld,
    ));

    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE as u32),
        BlockType::SIZE as u32,
        1,
        None,
        None,
    );

    commands
        .spawn((
            Name::new("Block Indicator"),
            SpriteBundle {
                texture: asset_server.load("textures/blocks.png"),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(TILE_SIZE as f32 * 0.75)),
                    anchor: bevy::sprite::Anchor::TopLeft,
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 3.0)),
                visibility: Visibility::Hidden,
                ..default()
            },
            TextureAtlas {
                layout: texture_atlas_layouts.add(layout),
                index: 0,
            },
            CursorBlockIcon,
            FromWorld,
        ))
        .with_children(|parent| {
            let new_layout = TextureAtlasLayout::from_grid(UVec2::splat(8), 2, 1, None, None);

            parent.spawn((
                SpriteBundle {
                    texture: asset_server.load("textures/place_modes.png"),
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(16.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(
                        TILE_SIZE as f32 * 0.75,
                        -(TILE_SIZE as f32 * 0.75),
                        4.0,
                    )),
                    ..default()
                },
                TextureAtlas {
                    layout: texture_atlas_layouts.add(new_layout),
                    index: 1,
                },
                CursorPlaceModeIcon,
            ));
        });
}

fn setup_sky_bg(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // clear color: 0.48, 0.48, 0.67

    let mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.5, -1.0, 0.0],  // 0
            [0.5, -1.0, 0.0],   // 1
            [0.5, 0.0, 0.0],    // 2
            [-0.5, 0.0, 0.0],   // 3
            [0.5, 0.025, 0.0],  // 4
            [-0.5, 0.025, 0.0], // 5
            [0.5, 1.0, 0.0],    // 6
            [-0.5, 1.0, 0.0],   // 7
            [0.5, 2.0, 0.0],    // 8
            [-0.5, 2.0, 0.0],   // 9
            [0.5, 3.0, 0.0],    // 10
            [-0.5, 3.0, 0.0],   // 11
        ],
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_COLOR,
        vec![
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0, 1.0],
            [0.48, 0.48, 0.67, 1.0],
            [0.48, 0.48, 0.67, 1.0],
            [0.125, 0.125, 1.0, 1.0],
            [0.125, 0.125, 1.0, 1.0],
            [0.0, 0.0, 0.5, 1.0],
            [0.0, 0.0, 0.5, 1.0],
            [0.0, 0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    )
    .with_inserted_indices(bevy::render::mesh::Indices::U32(vec![
        0, 1, 2, 2, 3, 0, 3, 2, 4, 4, 5, 3, 5, 4, 6, 6, 7, 5, 7, 6, 8, 8, 9, 7, 9, 8, 10, 10, 11, 9,
    ]));

    commands.spawn((
        Name::new("Sky Background"),
        MaterialMesh2dBundle {
            mesh: meshes.add(mesh).into(),
            material: materials.add(ColorMaterial::from_color(Srgba::new(0.7, 0.7, 1.0, 1.0))),
            //material: materials.add(ColorMaterial::default()),
            transform: Transform::from_xyz(0.0, 0.0, -2.0),
            ..default()
        },
        SkyBackground,
        FromWorld,
    ));
}

fn destroy_game(
    mut commands: Commands,
    world_q: Query<Entity, With<FromWorld>>,
    mut camera_q: Query<(&mut Camera, &mut Transform), With<MainCamera>>,
) {
    let (mut camera, mut camera_transform) = camera_q.single_mut();
    camera.clear_color = ClearColorConfig::Default;
    camera_transform.translation.x = 0.0;
    camera_transform.translation.y = 0.0;

    for entity in world_q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn pause_input(
    input: Res<ButtonInput<KeyCode>>,
    in_pause_state: Res<State<InPauseState>>,
    game_pause_state: Res<State<GamePauseState>>,
    mut next_game_pause_state: ResMut<NextState<GamePauseState>>,
    mut next_in_pause_state: ResMut<NextState<InPauseState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        if *game_pause_state.get() == GamePauseState::Paused {
            match in_pause_state.get() {
                InPauseState::Default => next_game_pause_state.set(GamePauseState::Running),
                _ => next_in_pause_state.set(InPauseState::Default),
            }
        } else {
            next_game_pause_state.set(GamePauseState::Paused);
        }
    }
}

fn on_game_paused(
    mut physics_time: ResMut<Time<Physics>>,
    mut cursor_q: Query<&mut Visibility, (With<BlockCursor>, Without<CursorBlockIcon>)>,
    mut cursor_icon_q: Query<&mut Visibility, (With<CursorBlockIcon>, Without<BlockCursor>)>,
) {
    physics_time.pause();

    if let Ok(mut vis) = cursor_q.get_single_mut() {
        *vis = Visibility::Hidden;
    }

    if let Ok(mut vis) = cursor_icon_q.get_single_mut() {
        *vis = Visibility::Hidden;
    }
}

fn on_game_unpaused(
    mut physics_time: ResMut<Time<Physics>>,
    mut cursor_q: Query<&mut Visibility, (With<BlockCursor>, Without<CursorBlockIcon>)>,
    mut cursor_icon_q: Query<&mut Visibility, (With<CursorBlockIcon>, Without<BlockCursor>)>,
) {
    physics_time.unpause();

    if let Ok(mut vis) = cursor_q.get_single_mut() {
        *vis = Visibility::Inherited;
    }

    if let Ok(mut vis) = cursor_icon_q.get_single_mut() {
        *vis = Visibility::Inherited;
    }
}

fn on_finished_saving_chunks(
    mut fin_save_chunks_ev: EventReader<FinishedSavingChunks>,
    mut state: ResMut<NextState<GameState>>,
) {
    for _ in fin_save_chunks_ev.read() {
        state.set(GameState::Menu);
    }
}

fn block_input(
    cursor_q: Query<&BlockCursor>,
    player_query: Query<&Transform, With<Player>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut try_place_block_ev: EventWriter<TryPlaceBlock>,
) {
    let cursor = cursor_q.single();
    let player_transform = player_query.single();

    let player_position = IVec2::new(
        (player_transform.translation.x / TILE_SIZE as f32).floor() as i32,
        (player_transform.translation.y / TILE_SIZE as f32).floor() as i32,
    );

    if cursor
        .block_position
        .as_vec2()
        .distance(player_position.as_vec2())
        > 7.0
    {
        return;
    }

    if player_position != cursor.block_position || cursor.layer == PlaceMode::WALL {
        if mouse_button_input.just_pressed(MouseButton::Right) {
            try_place_block_ev.send(TryPlaceBlock {
                position: cursor.relative_position,
                chunk_position: cursor.chunk_position,
                layer: cursor.layer,
                block_type: cursor.block_type,
            });
        }
    }
    if mouse_button_input.just_pressed(MouseButton::Left) {
        try_place_block_ev.send(TryPlaceBlock {
            position: cursor.relative_position,
            chunk_position: cursor.chunk_position,
            layer: cursor.layer,
            block_type: BlockType::AIR,
        });
    }
}

fn camera_follow_player(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<MainCamera>, Without<Player>)>,
) {
    if let Ok(player_transform) = player_query.get_single() {
        if let Ok(mut camera_transform) = camera_query.get_single_mut() {
            camera_transform.translation.x = lerp(
                camera_transform.translation.x,
                player_transform.translation.x,
                0.01,
            );
            camera_transform.translation.y = lerp(
                camera_transform.translation.y,
                player_transform.translation.y,
                0.01,
            );
        }
    }
}

fn update_sky_bg(
    window_query: Query<&Window, With<PrimaryWindow>>,
    mut sky_q: Query<&mut Transform, With<SkyBackground>>,
    camera_q: Query<&Transform, (With<MainCamera>, Without<SkyBackground>)>,
) {
    let Ok(window) = window_query.get_single() else {
        return;
    };
    let Ok(mut sky_transform) = sky_q.get_single_mut() else {
        return;
    };
    let Ok(camera_transform) = camera_q.get_single() else {
        return;
    };

    sky_transform.scale = Vec3::new(
        window.width() * camera_transform.scale.x,
        (CHUNK_AREA * TILE_SIZE) as f32,
        0.0,
    );

    sky_transform.translation.x = camera_transform.translation.x;
    //sky_transform.translation.y = camera_transform.translation.y - 200.0;
}

fn mouse_scroll_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    mut cursor_query: Query<&mut BlockCursor>,
    mut cursor_block_icon_q: Query<&mut TextureAtlas, With<CursorBlockIcon>>,
    mut mouse_scroll_event: EventReader<MouseWheel>,
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
            if ev.y > 0.0 {
                // Scrolling up
                if cursor.block_type < BlockType::GLASS {
                    cursor.block_type = enum_iterator::next(&cursor.block_type).unwrap();
                }
            } else if ev.y < 0.0 {
                // Scrolling down
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
    mut cursor_q: Query<(
        &mut Transform,
        &mut BlockCursor,
        &mut Sprite,
        &mut Visibility,
    )>,
    mut cursor_block_icon_q: Query<
        (&mut Transform, &mut Visibility),
        (With<CursorBlockIcon>, Without<BlockCursor>),
    >,
    camera_q: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    player_q: Query<&Transform, (With<Player>, Without<BlockCursor>, Without<CursorBlockIcon>)>,
    time: Res<Time>,
) {
    if let Ok(window) = window_query.get_single() {
        let (mut cursor_transform, mut cursor, mut cursor_sprite, mut cursor_visibility) =
            cursor_q.single_mut();
        let (mut cursor_icon_transform, mut cursor_icon_visibility) =
            cursor_block_icon_q.single_mut();
        let (camera, camera_global_transform) = camera_q.single();

        cursor_sprite
            .color
            .set_alpha((f32::sin(time.elapsed_seconds() * 4.0) / 4.0) + 0.25);

        if let Some(world_position) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world_2d(camera_global_transform, cursor))
        {
            *cursor_visibility = Visibility::Visible;

            if let Ok(player_transform) = player_q.get_single() {
                let player_position = IVec2::new(
                    (player_transform.translation.x / TILE_SIZE as f32).floor() as i32,
                    (player_transform.translation.y / TILE_SIZE as f32).floor() as i32,
                );
                if cursor
                    .block_position
                    .as_vec2()
                    .distance(player_position.as_vec2())
                    > 7.0
                {
                    *cursor_icon_visibility = Visibility::Hidden;
                } else {
                    *cursor_icon_visibility = Visibility::Visible;
                }
            } else {
                *cursor_icon_visibility = Visibility::Visible;
            }

            cursor_icon_transform.translation = Vec3::new(
                world_position.x,
                world_position.y,
                cursor_icon_transform.translation.z,
            );

            cursor.block_position = IVec2::new(
                (world_position.x as f32 / chunk::TILE_SIZE as f32).floor() as i32,
                (world_position.y as f32 / chunk::TILE_SIZE as f32).floor() as i32,
            );
            let g = Vec2::new(
                cursor.block_position.x as f32,
                cursor.block_position.y as f32,
            ) * TILE_SIZE as f32;
            cursor_transform.translation = Vec3::new(g.x, g.y, cursor_transform.translation.z);

            cursor.chunk_position = get_chunk_position(cursor.block_position);
            cursor.relative_position =
                get_relative_position(cursor.block_position, cursor.chunk_position);
        } else {
            *cursor_visibility = Visibility::Hidden;
            *cursor_icon_visibility = Visibility::Hidden;
        }
    }
}

fn switch_place_mode(
    mut cursor_q: Query<&mut BlockCursor>,
    mut cursor_placemode_icon_q: Query<&mut TextureAtlas, With<CursorPlaceModeIcon>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
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
    mut unload_chunks_ev: EventWriter<UnloadChunks>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        unload_chunks_ev.send(UnloadChunks { force: true });
    }
}
