use bevy::prelude::*;

use crate::{chunk::{Chunk, ChunkPlugin, PlaceBlock, PlaceMode, SpawnChunk, CHUNK_WIDTH, TILE_SIZE}, GameState};

#[derive(Event)]
pub struct TryPlaceBlock
{
    pub layer: PlaceMode,
    pub position: Vec2,
    pub id: u8,
}

pub struct ChunkManagerPlugin;

impl Plugin for ChunkManagerPlugin
{
    fn build(&self, app: &mut App) {
        app.add_event::<TryPlaceBlock>();
        app.add_plugins(ChunkPlugin);
        app.add_systems(Update, place_block_event.run_if(in_state(GameState::Game)));
    }
}

fn place_block_event(
    chunk_query: Query<(&Transform, &Children), With<Chunk>>,
    mut try_place_block_ev: EventReader<TryPlaceBlock>,
    mut place_block_ev: EventWriter<PlaceBlock>,
    mut spawn_chunk_ev: EventWriter<SpawnChunk>
) {
    for ev in try_place_block_ev.read() {
        let cursor_position = ev.position;
        let chunk_position = ((cursor_position / TILE_SIZE as f32) / CHUNK_WIDTH as f32).floor();
        let relative_pos = (cursor_position / TILE_SIZE as f32).floor() - (chunk_position * CHUNK_WIDTH as f32);
    
        for (transform, children) in chunk_query.iter()
        {
            if Vec2::new(transform.translation.x, transform.translation.y) == ((chunk_position * CHUNK_WIDTH as f32) * TILE_SIZE as f32) {
                place_block_ev.send(PlaceBlock{
                    layer: ev.layer,
                    position: UVec2::new(relative_pos.x as u32, relative_pos.y as u32),
                    id: ev.id,
                    entity: children[ev.layer as usize]
                });
                return;
            }
        }
        
        if ev.id > 0 {
            spawn_chunk_ev.send(SpawnChunk {
                position: IVec2::new(chunk_position.x as i32, chunk_position.y as i32),
                place_block: Some((ev.layer, 1, UVec2::new(relative_pos.x as u32, relative_pos.y as u32)))
            });
        }
    }
}