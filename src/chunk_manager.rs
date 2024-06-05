use bevy::{math::Vec3A, prelude::*, render::primitives::Aabb, sprite::{Anchor, MaterialMesh2dBundle}, utils::HashMap, window::PrimaryWindow};
use bevy_xpbd_2d::components::RigidBody;

use crate::{chunk::{generate_chunk_layer_mesh, CalcLightChunks, Chunk, ChunkComponent, ChunkLayer, ChunkPlugin, PlaceMode, RecollisionChunk, RemeshChunks, CHUNK_AREA, CHUNK_WIDTH, TILE_SIZE}, utils::*, world::{GameSystemSet, WorldGenPreset, WorldInfo}};

#[derive(Event)]
pub struct TryPlaceBlock
{
    pub layer: PlaceMode,
    pub position: IVec2,
    pub id: u8,
}

#[derive(Event)]
pub struct UnloadChunks
{
    pub force: bool
}

#[derive(Event)]
pub struct LoadChunks;

#[derive(Event)]
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
        app.insert_resource(Chunks(HashMap::new()));
        app.add_plugins(ChunkPlugin);
        app.add_systems(Update, (load_chunks, spawn_chunk, try_to_place_block_event).chain().in_set(GameSystemSet::ChunkManager));
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

        let pixel_chunk_pos = Vec2::new((ev.position.x as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32, (ev.position.y as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32);
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

            if ev.id > 0 {
                if chunk.layers[ev.layer as usize][index] <= 0 {
                    match ev.layer {
                        PlaceMode::BLOCK => {
                            if let Some(bn) = block_neighbors {
                                if let Some(wn) = wall_neighbors {
                                    if wn[0] > 0 ||
                                        bn[1] > 0 || bn[2] > 0 || bn[3] > 0 || bn[4] > 0
                                    {
                                        chunk.layers[PlaceMode::BLOCK as usize][index] = ev.id;
                                    }
                                }
                            }
                        },
                        PlaceMode::WALL => {
                            if let Some(bn) = block_neighbors {
                                if let Some(wn) = wall_neighbors {
                                    if bn[0] > 0 || bn[1] > 0 || bn[2] > 0 || bn[3] > 0 || bn[4] > 0 ||
                                        wn[1] > 0 || wn[2] > 0 || wn[3] > 0 || wn[4] > 0
                                    {
                                        chunk.layers[PlaceMode::WALL as usize][index] = ev.id;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            else
            {
                if chunk.layers[ev.layer as usize][index] > 0 {
                    chunk.layers[ev.layer as usize][index] = 0;
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

/*
fn unload_and_save_chunks(
    mut unload_chunks_ev : EventReader<UnloadChunks>,
    mut load_chunks_ev: EventWriter<LoadChunks>,
    mut commands: Commands,
    chunk_query: Query<(Entity, &Chunk, &Children, &ViewVisibility)>,
    chunk_layer_query: Query<&ChunkLayer>,
    world_info_res: Res<WorldInfo>
) {
    for ev in unload_chunks_ev.read() {
        // ===================================
        // Despawn and save chunks out of view
        for (c_entity, chunk, c_children, c_visibility) in chunk_query.iter() {
            // If a chunk is out of view, then save its blocks into a file and despawn the chunk entity
            // Or if force bool is true (unload and load all chunks no matter if its already there)
            if !c_visibility.get() || ev.force {
                let mut layers: [serde_big_array::Array<u8, CHUNK_AREA>; 2] = [serde_big_array::Array([0; CHUNK_AREA]); 2];
                for c in 0..c_children.len() {
                    if chunk_layer_query.contains(c_children[c]) {
                        let chunk_layer = chunk_layer_query.get(c_children[c]).unwrap();
                        layers[c] = serde_big_array::Array(chunk_layer.blocks.clone());
                    } else { break; }
                }
                let s = bincode::serialize(&layers).unwrap();
                let a = chunk.position;
                let world_name = world_info_res.name.clone();
                match std::fs::write(format!("worlds/{}/chunks/{}.bin", world_name, a), &s) {
                    Err(e) => println!("Error saving chunk at {}: {}", a, e),
                    _ => {}
                }
                /*
                IoTaskPool::get()
                .spawn(async move {
                    match std::fs::write(format!("worlds/{}/chunks/{}.bin", world_name, a), &s) {
                        Err(e) => println!("Error saving chunk at {}: {}", a, e),
                        _ => {}
                    }
                }).detach();
                */
                commands.entity(c_entity).despawn_recursive();
            }
        }
        
        load_chunks_ev.send(LoadChunks {});
    }
}
*/

fn load_chunks(
    mut chunks_res: ResMut<Chunks>,
    mut load_chunks_ev: EventReader<LoadChunks>,
    mut spawn_chunk_ev: EventWriter<SpawnChunk>,
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
            
            // Had to make it load some extra chunks offscreen
            // to make it truly seamless
            for y in (c_bottom_right.y-1)..(c_top_left.y+2) {
                for x in (c_top_left.x-1)..(c_bottom_right.x+2) {
                    let pos = IVec2::new(x, y);
                    let already_has: bool = chunks_res.contains_key(&pos);
                    
                    if !already_has {
                        let str = format!("worlds/{}/chunks/{}.bin", world_info_res.name, pos);
                        let path = std::path::Path::new(str.as_str());
                        
                        let mut blocks: [u8; CHUNK_AREA] = [0; CHUNK_AREA];
                        let mut walls: [u8; CHUNK_AREA] = [0; CHUNK_AREA];
                        
                        if path.exists() {
                            match std::fs::read(str) {
                                Ok(bytes) => {
                                    let layers: [serde_big_array::Array<u8, CHUNK_AREA>; 2] = bincode::deserialize(&bytes).unwrap();
                                    blocks = layers[1].0;
                                    walls = layers[0].0;
                                },
                                Err(e) => println!("Error when trying to load chunk at {}: {}", pos, e)
                            }
                        } else {
                            // TODO: This is where world generation goes in!
                            match world_info_res.preset {
                                WorldGenPreset::EMPTY => {
                                    if pos == IVec2::ZERO {
                                        for x in 0..(CHUNK_WIDTH/2) {
                                            blocks[x] = 1;
                                        }
                                    } else if pos == IVec2::new(-1, 0) {
                                        for x in 0..(CHUNK_WIDTH/2) {
                                            blocks[(CHUNK_WIDTH/2)+x] = 1;
                                        }
                                    }
                                },
                                WorldGenPreset::FLAT => {
                                    if pos.y == 0 {
                                        for y in 0..CHUNK_WIDTH {
                                            for x in 0..CHUNK_WIDTH {
                                                if y == CHUNK_WIDTH/2 {
                                                    blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = 1;
                                                }
                                                else if y < CHUNK_WIDTH/2 {
                                                    blocks[get_index_from_position(UVec2::new(x as u32, y as u32))] = 2;
                                                }
                                            }
                                        }
                                    }
                                    else if pos.y < 0 && pos.y >= -2 {
                                        blocks = [2; CHUNK_AREA];
                                    }
                                    else if pos.y < -2 {
                                        blocks = [3; CHUNK_AREA];
                                    }
                                }
                                _ => {}
                            }
                        }
                        
                        chunks_res.insert(pos, Chunk { layers: [walls, blocks], light: [0; CHUNK_AREA] });
                        spawn_chunk_ev.send(SpawnChunk{ position: pos });
                    }
                }
            }
        }
    }
}