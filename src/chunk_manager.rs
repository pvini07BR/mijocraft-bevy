use bevy::{math::Vec3A, prelude::*, render::primitives::Aabb, sprite::{Anchor, MaterialMesh2dBundle}, tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, IoTaskPool, Task}, utils::HashMap, window::PrimaryWindow};
use bevy_xpbd_2d::components::RigidBody;
use std::io::ErrorKind;

use crate::{player::Player, world::FromWorld, GameState};
use crate::{chunk::{generate_chunk_layer_mesh, BlockType, CalcLightChunks, Chunk, ChunkComponent, ChunkLayer, ChunkPlugin, PlaceMode, RecollisionChunk, RemeshChunks, CHUNK_AREA, CHUNK_WIDTH, TILE_SIZE}, utils::*, world::{GameSystemSet, WorldGenPreset, WorldInfo}};

#[derive(Event)]
pub struct TryPlaceBlock
{
    pub layer: PlaceMode,
    pub position: IVec2,
    pub id: BlockType,
}

#[derive(Event)]
pub struct UnloadChunks
{
    pub force: bool
}

#[derive(Event)]
pub struct LoadChunks;

#[derive(Event)]
pub struct SaveAllChunks;

#[derive(Event)]
pub struct FinishedSavingChunks;

#[derive(Component)]
struct ComputeChunkLoading(Task<(Chunk, SpawnChunk)>);

#[derive(Event, Debug)]
pub struct SpawnChunk {
    pub position: IVec2
}

#[derive(Resource, DerefMut, Deref)]
pub struct Chunks(pub HashMap<IVec2, Chunk>);

pub struct ChunkManagerPlugin;

impl Plugin for ChunkManagerPlugin
{
    fn build(&self, app: &mut App) {
        app.add_event::<TryPlaceBlock>();
        app.add_event::<SpawnChunk>();
        app.add_event::<UnloadChunks>();
        app.add_event::<LoadChunks>();
        app.add_event::<SaveAllChunks>();
        app.add_event::<FinishedSavingChunks>();

        app.insert_resource(Chunks(HashMap::new()));
        app.add_plugins(ChunkPlugin);
        app.add_systems(Update, (
            unload_and_save_chunks,
            load_chunks,
            process_chunk_loading_tasks,
            spawn_chunk,
            try_to_place_block_event,
            save_all_chunks
        ).chain().in_set(GameSystemSet::ChunkManager));
        app.add_systems(OnExit(GameState::Game), clear_chunks);
    }
}

pub fn spawn_chunk(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut spawn_chunk_ev: EventReader<SpawnChunk>,
    mut calc_light_ev: EventWriter<CalcLightChunks>,
    mut remesh_chunk_ev: EventWriter<RemeshChunks>,
    mut recol_chunk_ev: EventWriter<RecollisionChunk>,
    asset_server: Res<AssetServer>
) {
    for ev in spawn_chunk_ev.read() {

        let pixel_chunk_pos = ev.position.as_vec2() * CHUNK_WIDTH as f32 * TILE_SIZE as f32;
        let chunk_material_handle = materials.add(asset_server.load("textures/blocks.png"));
        
        let id = commands.spawn(
            (
                Name::new(format!("Chunk at ({}, {})", ev.position.x, ev.position.y)),
                RigidBody::Static,
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                        anchor: Anchor::BottomLeft,
                        custom_size: Some(Vec2::splat(CHUNK_WIDTH as f32 * TILE_SIZE as f32)),
                        ..default()
                    },
                    transform: Transform::from_xyz(pixel_chunk_pos.x, pixel_chunk_pos.y, 0.0),
                    ..default()
                },
                Aabb {
                    center: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                    half_extents: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                },
                ShowAabbGizmo {..default()},
                ChunkComponent {position: ev.position},
                FromWorld
            )
        ).with_children(|parent| {
            parent.spawn((
                Name::new("Chunk Wall Layer"),
                MaterialMesh2dBundle {
                    mesh: meshes.add(generate_chunk_layer_mesh()).into(),
                    material: chunk_material_handle.clone(),
                    transform: Transform::from_xyz(0.0, 0.0, -1.0),
                    ..default()
                },
                Aabb {
                    center: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                    half_extents: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                },
                ChunkLayer
            ));

            parent.spawn((
                Name::new("Chunk Block Layer"),
                MaterialMesh2dBundle {
                    mesh: meshes.add(generate_chunk_layer_mesh()).into(),
                    material: chunk_material_handle,
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                },
                Aabb {
                    center: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                    half_extents: Vec3A::splat(CHUNK_WIDTH as f32 / 2.0) * TILE_SIZE as f32,
                },
                ChunkLayer
            ));
        }).id();

        calc_light_ev.send(CalcLightChunks);
        remesh_chunk_ev.send(RemeshChunks);
        recol_chunk_ev.send(RecollisionChunk { entity: id });
    }
}

fn try_to_place_block_event(
    mut chunks_res: ResMut<Chunks>,
    mut try_place_block_ev: EventReader<TryPlaceBlock>,
    mut calc_light_ev: EventWriter<CalcLightChunks>,
    mut remesh_chunk_ev: EventWriter<RemeshChunks>,
    mut recol_chunk_ev: EventWriter<RecollisionChunk>,
    chunk_query: Query<(Entity, &ChunkComponent)>
) {
    for ev in try_place_block_ev.read() {
        let chunk_position = get_chunk_position(ev.position);
        let relative_pos = get_relative_position(ev.position, chunk_position);
        let block_neighbors = get_neighboring_blocks(&chunks_res, ev.position, PlaceMode::BLOCK);
        let wall_neighbors = get_neighboring_blocks(&chunks_res, ev.position, PlaceMode::WALL);

        if let Some(chunk) = chunks_res.get_mut(&chunk_position) {
            let index = get_index_from_position(relative_pos);

            if ev.id > BlockType::AIR {
                if chunk.layers[ev.layer as usize][index] <= BlockType::AIR {
                    match ev.layer {
                        PlaceMode::BLOCK => {
                            if let (Some(bn), Some(wn)) = (block_neighbors, wall_neighbors) {
                                if wn[0] > BlockType::AIR ||
                                    bn[1] > BlockType::AIR || bn[2] > BlockType::AIR || bn[3] > BlockType::AIR || bn[4] > BlockType::AIR
                                {
                                    chunk.layers[PlaceMode::BLOCK as usize][index] = ev.id;
                                }
                            }
                        },
                        PlaceMode::WALL => {
                            if let (Some(bn), Some(wn)) = (block_neighbors, wall_neighbors) {
                                if bn[0] > BlockType::AIR || bn[1] > BlockType::AIR || bn[2] > BlockType::AIR || bn[3] > BlockType::AIR || bn[4] > BlockType::AIR ||
                                    wn[1] > BlockType::AIR || wn[2] > BlockType::AIR || wn[3] > BlockType::AIR || wn[4] > BlockType::AIR
                                {
                                    chunk.layers[PlaceMode::WALL as usize][index] = ev.id;
                                }
                            }
                        }
                    }
                }
            }
            else
            {
                if chunk.layers[ev.layer as usize][index] > BlockType::AIR {
                    chunk.layers[ev.layer as usize][index] = BlockType::AIR;
                }
            }

            calc_light_ev.send(CalcLightChunks);
            remesh_chunk_ev.send(RemeshChunks);
            for (entity, chunk_compo) in chunk_query.iter() {
                if chunk_compo.position == chunk_position {
                    recol_chunk_ev.send(RecollisionChunk { entity });
                }
            }
        }

    }
}

fn save_all_chunks(
    player_q: Query<&Transform, With<Player>>,
    mut save_chunks_ev: EventReader<SaveAllChunks>,
    mut finished_saving_ev: EventWriter<FinishedSavingChunks>,
    chunks_res: Res<Chunks>,
    world_info_res: Res<WorldInfo>
) {
    for _ in save_chunks_ev.read() {
        info!("Saving all chunks...");

        for (pos, chunk) in chunks_res.iter() {
            let mut layers: [serde_big_array::Array<BlockType, CHUNK_AREA>; 2] = [serde_big_array::Array([BlockType::AIR; CHUNK_AREA]); 2];
            
            layers[0] = serde_big_array::Array(chunk.layers[0].clone());
            layers[1] = serde_big_array::Array(chunk.layers[1].clone());

            match bincode::serialize(&layers) {
                Ok(s) => {
                    let a = pos;
                    let world_name = world_info_res.name.clone();
                    match std::fs::write(format!("worlds/{}/chunks/{}.bin", world_name, a), &s) {
                        Err(e) => error!("Error saving chunk at {}: {}", a, e),
                        _ => {}
                    }
                },
                Err(e) => error!("Could not serialize chunk at {}: {}", pos, e)
            }
        }

        if let Ok(player_transform) = player_q.get_single() {
            let mut new_info = world_info_res.clone();
            new_info.last_player_pos = player_transform.translation.xy();

            if let Ok(str) = toml::to_string(&new_info) {
                let _ = std::fs::write(format!("worlds/{}/world.toml", new_info.name), str);
            }
        }

        finished_saving_ev.send(FinishedSavingChunks);
    }
}

fn unload_and_save_chunks(
    mut commands: Commands,
    mut chunks: ResMut<Chunks>,
    mut unload_chunks_ev: EventReader<UnloadChunks>,
    mut load_chunks_ev: EventWriter<LoadChunks>,
    chunk_query: Query<(Entity, &ChunkComponent, &ViewVisibility)>,
    world_info_res: Res<WorldInfo>
) {
    for ev in unload_chunks_ev.read() {
        // ===================================
        // Despawn and save chunks out of view
        for (chunk_entity, chunk_compo, chunk_visibility) in chunk_query.iter() {
            // If a chunk is out of view, then save its blocks into a file and despawn the chunk entity
            // Or if force bool is true (unload and load all chunks no matter if its already there)
            if !chunk_visibility.get() || ev.force {
                let mut layers: [serde_big_array::Array<BlockType, CHUNK_AREA>; 2] = [serde_big_array::Array([BlockType::AIR; CHUNK_AREA]); 2];
                if let Some(chunk) = chunks.get(&chunk_compo.position) {
                    layers[0] = serde_big_array::Array(chunk.layers[0].clone());
                    layers[1] = serde_big_array::Array(chunk.layers[1].clone());
    
                    let a = chunk_compo.position;

                    match bincode::serialize(&layers) {
                        Ok(s) => {
                            let world_name = world_info_res.name.clone();
                            IoTaskPool::get().spawn(async move {
                                match std::fs::write(format!("worlds/{}/chunks/{}.bin", world_name, a), &s) {
                                    Err(e) => error!("Error saving chunk at {}: {}", a, e),
                                    _ => {}
                                }
                            }).detach();
                        },
                        Err(e) => error!("Could not serialize chunk at {}: {}", a, e)
                    }
    
                    chunks.remove(&chunk_compo.position);
                    commands.entity(chunk_entity).despawn_recursive();
                }
            }
        }

        load_chunks_ev.send(LoadChunks {});
    }
}

fn load_chunks(
    mut commands: Commands,
    mut load_chunks_ev: EventReader<LoadChunks>,
    chunks_res: ResMut<Chunks>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    world_info_res: Res<WorldInfo>
) {
    // ==========================
    // Load chunks from disk
    for _ in load_chunks_ev.read() {
        if let Ok(window) = window_query.get_single() {
            let (camera, camera_global_transform) = camera_query.get_single().unwrap();
    
            let top_left = camera.viewport_to_world_2d(camera_global_transform, Vec2::new(0.0, 0.0)).unwrap();
            let bottom_right = camera.viewport_to_world_2d(camera_global_transform, Vec2::new(window.width(), window.height())).unwrap();
    
            let b_top_left = get_block_position(top_left);
            let b_bottom_right = get_block_position(bottom_right);
    
            let c_top_left = get_chunk_position(b_top_left);
            let c_bottom_right = get_chunk_position(b_bottom_right);

            let world_name = world_info_res.name.clone();
            let world_preset = world_info_res.preset.clone();

            // Had to make it load some extra chunks offscreen
            // to make it truly seamless
            for y in (c_bottom_right.y-1)..(c_top_left.y+2) {
                for x in (c_top_left.x-1)..(c_bottom_right.x+2) {
                    let pos = IVec2::new(x, y);
                    let already_has: bool = chunks_res.contains_key(&pos);
                    let str = format!("worlds/{}/chunks/{}.bin", world_name, pos);

                    let thread_pool = AsyncComputeTaskPool::get();
                    if !already_has {
                        let task_entity = commands.spawn_empty().id();
                        commands.entity(task_entity).insert(Name::new("Chunk Loading Async Task"));
                        commands.entity(task_entity).insert(FromWorld);

                        let task: Task<(Chunk, SpawnChunk)> = thread_pool.spawn(async move {
                            let mut blocks: [BlockType; CHUNK_AREA] = [BlockType::AIR; CHUNK_AREA];
                            let mut walls: [BlockType; CHUNK_AREA] = [BlockType::AIR; CHUNK_AREA];
                            
                            match std::fs::read(str) {
                                Ok(bytes) => {
                                    match bincode::deserialize::<[serde_big_array::Array<BlockType, CHUNK_AREA>; 2]>(&bytes) {
                                        Ok(layers) => {
                                            blocks = layers[1].0;
                                            walls = layers[0].0;
                                        },
                                        Err(e) => error!("Error deserializing chunk at {}: {}", pos, e)
                                    }
                                },
                                Err(e) => {
                                    if e.kind() != ErrorKind::NotFound {
                                        error!("Error when trying to load chunk at {}: {}", pos, e);
                                    } else {
                                        // If a chunk file is not found at a certain location,
                                        // then it will try to generate a new one from scratch.
                                        // This is where world generation goes in!

                                        match world_preset {
                                            WorldGenPreset::DEFAULT => {
                                                if pos.y == 0 {
                                                    for x in 0..CHUNK_WIDTH {
                                                        let mut s = (f32::sin((x as f32 + (pos.x as f32 * CHUNK_WIDTH as f32)) / 6.0) / 2.0) + 0.5;
                                                        s *= CHUNK_WIDTH as f32 / 2.0;
                                                        for y in 0..CHUNK_WIDTH {
                                                            if y as f32 > s {
                                                                blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::AIR;
                                                            } else {
                                                                blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::STONE;
                                                            }
                                                        }
                                                    }
                                                } else if pos.y < 0 {
                                                    for y in 0..CHUNK_WIDTH {
                                                        for x in 0..CHUNK_WIDTH {
                                                            blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::STONE;
                                                        }
                                                    }
                                                }
                                            },

                                            WorldGenPreset::EMPTY => {
                                                if pos == IVec2::ZERO {
                                                    for x in 0..(CHUNK_WIDTH/2) {
                                                        blocks[x] = BlockType::GRASS;
                                                    }
                                                } else if pos == IVec2::new(-1, 0) {
                                                    for x in 0..(CHUNK_WIDTH/2) {
                                                        blocks[(CHUNK_WIDTH/2)+x] = BlockType::GRASS;
                                                    }
                                                }
                                            },
                                            WorldGenPreset::FLAT => {
                                                if pos.y == 0 {
                                                    for y in 0..CHUNK_WIDTH {
                                                        for x in 0..CHUNK_WIDTH {
                                                            if y == CHUNK_WIDTH/2 {
                                                                blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::GRASS;
                                                                walls[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::GRASS;
                                                            }
                                                            else if y < CHUNK_WIDTH/2 {
                                                                blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::DIRT;
                                                                walls[get_index_from_position(UVec2::new(x as u32, y as u32))] = BlockType::DIRT;
                                                            }
                                                        }
                                                    }
                                                }
                                                else if pos.y < 0 && pos.y >= -2 {
                                                    blocks = [BlockType::DIRT; CHUNK_AREA];
                                                    walls = [BlockType::DIRT; CHUNK_AREA];
                                                }
                                                else if pos.y < -2 {
                                                    blocks = [BlockType::STONE; CHUNK_AREA];
                                                    walls = [BlockType::STONE; CHUNK_AREA];
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            return (Chunk { layers: [walls, blocks], light: [0; CHUNK_AREA] }, SpawnChunk{ position: pos });
                        });

                        commands.entity(task_entity).insert(ComputeChunkLoading(task));
                    }
                }
            }
        }
    }
}

fn process_chunk_loading_tasks(
    mut commands: Commands,
    mut tasks_query: Query<(Entity, &mut ComputeChunkLoading)>,
    mut spawn_chunk_ev: EventWriter<SpawnChunk>,
    mut chunks_res: ResMut<Chunks>,
) {
    for (entity, mut task) in tasks_query.iter_mut() {
        if let Some((chunk, spawn_chunk)) = block_on(future::poll_once(&mut task.0)) {
            chunks_res.insert(spawn_chunk.position, chunk);
            spawn_chunk_ev.send(spawn_chunk);
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn clear_chunks(
    mut chunks_res: ResMut<Chunks>
) {
    chunks_res.clear();
}