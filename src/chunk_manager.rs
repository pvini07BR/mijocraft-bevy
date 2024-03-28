use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{chunk::{Chunk, ChunkLayer, ChunkPlugin, PlaceBlock, PlaceMode, SpawnChunk, CHUNK_AREA, CHUNK_WIDTH, TILE_SIZE}, utils::*, world::GameSystemSet};

#[derive(Event)]
pub struct TryPlaceBlock
{
    pub layer: PlaceMode,
    pub position: IVec2,
    pub id: u8,
}

#[derive(SystemParam)]
pub struct GetBlockSysParam<'w, 's> {
    pub chunk_query: Query<'w, 's, (&'static Transform, &'static Children), With<Chunk>>,
    pub chunk_layer_query: Query<'w, 's, &'static ChunkLayer>
}

pub struct ChunkManagerPlugin;

impl Plugin for ChunkManagerPlugin
{
    fn build(&self, app: &mut App) {
        app.add_event::<TryPlaceBlock>();
        app.add_plugins(ChunkPlugin);
        app.add_systems(Update, place_block_event.in_set(GameSystemSet::ChunkManager));
    }
}

fn place_block_event(
    chunk_query: Query<(&Transform, Entity, &Children), With<Chunk>>,
    chunk_layer_query: Query<&ChunkLayer>,
    mut try_place_block_ev: EventReader<TryPlaceBlock>,
    mut place_block_ev: EventWriter<PlaceBlock>,
    mut spawn_chunk_ev: EventWriter<SpawnChunk>
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

        for (transform, chunk_entity, chunk_children) in chunk_query.iter()
        {
            let to_pixel_pos = Vec2::new((chunk_position.x as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32, (chunk_position.y as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32);
            if transform.translation.xy() == to_pixel_pos {
                let chunk_layer = chunk_layer_query.get(chunk_children[p.layer as usize]).unwrap();
                if chunk_layer.blocks[get_index_from_position(p.position)] <= 0 && p.id > 0 || chunk_layer.blocks[get_index_from_position(p.position)] > 0 && p.id <= 0 {
                    p.entity = chunk_entity;
                    place_block_ev.send(p);
                }
                return;
            }
        }

        if p.id > 0 {
            spawn_chunk_ev.send(SpawnChunk {
                position: IVec2::new(chunk_position.x as i32, chunk_position.y as i32),
                place_block: Some(p)
            });
        }
    }
}

pub fn get_block(sys_param: &mut GetBlockSysParam<'_, '_>, relative_coords: IVec2, chunk_position: IVec2, place_mode: PlaceMode, blocks: &[u8; CHUNK_AREA]) -> u8 {
    // First check if the relative coordinates are inside the same chunk
    if relative_coord_is_inside_bounds(relative_coords) {
        let uvec = UVec2::new(relative_coords.x as u32, relative_coords.y as u32);
        return blocks[get_index_from_position(uvec)];
    }
    else {
        // If not, then start checking which chunk it belongs to
        for (chunk_transform, chunk_children) in sys_param.chunk_query.iter() {
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