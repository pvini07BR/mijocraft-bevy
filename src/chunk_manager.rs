use bevy::{ecs::system::SystemParam, prelude::*, window::PrimaryWindow};

use crate::{chunk::{Chunk, ChunkLayer, ChunkPlugin, PlaceBlock, PlaceMode, SpawnChunk, CHUNK_AREA, CHUNK_WIDTH}, menu::WorldInfo, utils::*, world::GameSystemSet};

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

#[derive(SystemParam)]
pub struct GetBlockSysParam<'w, 's> {
    pub chunk_query: Query<'w, 's, (Entity, &'static Transform, &'static Children), With<Chunk>>,
    pub chunk_layer_query: Query<'w, 's, &'static ChunkLayer>
}

pub struct ChunkManagerPlugin;

impl Plugin for ChunkManagerPlugin
{
    fn build(&self, app: &mut App) {
        app.add_event::<TryPlaceBlock>();
        app.add_event::<UnloadChunks>();
        app.add_event::<LoadChunks>();
        app.add_plugins(ChunkPlugin);
        app.add_systems(Update, (unload_and_save_chunks, load_chunks, place_block_event).chain().in_set(GameSystemSet::ChunkManager));
    }
}

fn place_block_event(
    mut sys_param: GetBlockSysParam<'_, '_>,
    mut try_place_block_ev: EventReader<TryPlaceBlock>,
    mut place_block_ev: EventWriter<PlaceBlock>
) {
    for ev in try_place_block_ev.read() {
        let chunk_position = get_chunk_position(ev.position);
        let relative_pos = get_relative_position(ev.position, chunk_position);
    
        let mut p = PlaceBlock{
            layer: ev.layer,
            position: UVec2::new(relative_pos.x as u32, relative_pos.y as u32),
            id: ev.id,
            entity: Entity::PLACEHOLDER
        };

        for (chunk_entity, transform, chunk_children) in sys_param.chunk_query.iter()
        {
            if get_chunk_position_from_translation(transform.translation.xy()) == chunk_position {
                p.entity = chunk_entity;

                let chunk_layer = sys_param.chunk_layer_query.get(chunk_children[p.layer as usize]).unwrap();
                if chunk_layer.blocks[get_index_from_position(p.position)] <= 0 && p.id > 0 || chunk_layer.blocks[get_index_from_position(p.position)] > 0 && p.id <= 0 {
                    // Also check neighbors when placing a block
                    if p.id > 0 {
                        // For blocks, just check if there is a neighboring block (in the same layer only),
                        // also if there is a wall on the same position

                        if ev.layer == PlaceMode::BLOCK {
                            let wall = lazy_get_block(&mut sys_param, ev.position, PlaceMode::WALL);

                            let left = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_X, PlaceMode::BLOCK);
                            let up = lazy_get_block(&mut sys_param, ev.position + IVec2::Y, PlaceMode::BLOCK);
                            let right = lazy_get_block(&mut sys_param, ev.position + IVec2::X, PlaceMode::BLOCK);
                            let down = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_Y, PlaceMode::BLOCK);
    
                            if !(wall > 0 || left > 0 || up > 0 || right > 0 || down > 0) { return; }
                        
                        // For walls, check for neighbors in walls AND blocks (both layers!)
                        } else if ev.layer == PlaceMode::WALL {
                            let b_left = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_X, PlaceMode::BLOCK);
                            let w_left = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_X, PlaceMode::WALL);

                            let b_up = lazy_get_block(&mut sys_param, ev.position + IVec2::Y, PlaceMode::BLOCK);
                            let w_up = lazy_get_block(&mut sys_param, ev.position + IVec2::Y, PlaceMode::WALL);

                            let b_right = lazy_get_block(&mut sys_param, ev.position + IVec2::X, PlaceMode::BLOCK);
                            let w_right = lazy_get_block(&mut sys_param, ev.position + IVec2::X, PlaceMode::WALL);

                            let b_down = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_Y, PlaceMode::BLOCK);
                            let w_down = lazy_get_block(&mut sys_param, ev.position + IVec2::NEG_Y, PlaceMode::WALL);
    
                            if !(b_left > 0 || b_up > 0 || b_right > 0 || b_down > 0 || w_left > 0 || w_up > 0 || w_right > 0 || w_down > 0) { return; }
                        }
                    }

                    place_block_ev.send(p);
                }
                return;
            }
        }
    }
}

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

fn load_chunks(
    mut load_chunks_ev: EventReader<LoadChunks>,
    mut spawn_chunk_ev: EventWriter<SpawnChunk>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    chunk_query: Query<&Chunk>,
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
    
            for y in c_bottom_right.y..(c_top_left.y+1) {
                for x in c_top_left.x..(c_bottom_right.x+1) {
                    let pos = IVec2::new(x, y);
                    let mut already_has: bool = false;
                    
                    for chunk in chunk_query.iter() {
                        if chunk.position == pos {
                            already_has = true;
                            break;
                        }
                    }
                    
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
                        }
                        
                        // TODO: This is where world generation goes in!
                        spawn_chunk_ev.send(SpawnChunk { position: pos, blocks: blocks, walls: walls });
                    }
                }
            }
        }
    }
}

pub fn lazy_get_block(
    sys_param: &mut GetBlockSysParam<'_, '_>,
    block_coords: IVec2,
    place_mode: PlaceMode
) -> u8 {
    let chunk_coords = get_chunk_position(block_coords);

    for (_, chunk_transform, chunk_children) in sys_param.chunk_query.iter() {
        let chunk_pos = get_chunk_position_from_translation(chunk_transform.translation.xy());
        if chunk_pos == chunk_coords {
            if let Ok(chunk_layer) = sys_param.chunk_layer_query.get(chunk_children[place_mode as usize]) {
                let relative = get_relative_position(block_coords, chunk_coords);
                return chunk_layer.blocks[get_index_from_position(relative)];
            }
        }
    }

    return 0;
}

pub fn get_block(sys_param: &mut GetBlockSysParam<'_, '_>, relative_coords: IVec2, chunk_position: IVec2, place_mode: PlaceMode, blocks: &[u8; CHUNK_AREA]) -> u8 {
    // First check if the relative coordinates are inside the same chunk
    if relative_coord_is_inside_bounds(relative_coords) {
        let uvec = UVec2::new(relative_coords.x as u32, relative_coords.y as u32);
        return blocks[get_index_from_position(uvec)];
    }
    else {
        // If not, then start checking which chunk it belongs to
        for (_, chunk_transform, chunk_children) in sys_param.chunk_query.iter() {
            let chunk_pos = get_chunk_position_from_translation(chunk_transform.translation.xy());
            if chunk_pos == chunk_position { continue; }
            if chunk_pos == (chunk_position + get_chunk_diff(relative_coords)) {
                if let Ok(chunk_layer) = sys_param.chunk_layer_query.get(chunk_children[place_mode as usize]) {
                    let fixed_pos = UVec2::new(
                        modular(relative_coords.x, CHUNK_WIDTH as i32) as u32,
                        modular(relative_coords.y, CHUNK_WIDTH as i32) as u32
                    );
                    return chunk_layer.blocks[get_index_from_position(fixed_pos)];
                }
            }
        }

        return 0;
    }
}