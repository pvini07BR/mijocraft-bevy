pub const TILE_SIZE: usize = 32;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_AREA: usize = CHUNK_WIDTH*CHUNK_WIDTH;

const VERTICES_PER_BLOCK: usize = 4;
const INDICES_PER_BLOCK: usize = 6;

const CHUNK_MESH_SIZE: usize = CHUNK_AREA * VERTICES_PER_BLOCK;
const CHUNK_INDEX_COUNT: usize = CHUNK_AREA * INDICES_PER_BLOCK;

use bevy::{prelude::*, render::{mesh::{Indices, PrimitiveTopology}, render_asset::RenderAssetUsages}, sprite::Mesh2dHandle};
use bevy_xpbd_2d::plugins::collision::Collider;
use enum_iterator::Sequence;

use crate::{chunk_manager::Chunks, utils::{get_global_position, get_index_from_position, get_neighboring_lights, get_position_from_index}, world::GameSystemSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaceMode {
    WALL = 0,
    BLOCK = 1
}

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, PartialOrd, Serialize, Deserialize, Sequence)]
pub enum BlockType {
    #[default]
    AIR = 0,
    GRASS,
    DIRT,
    STONE,
    PLANKS,
    GLASS,
    SIZE
}

impl BlockType {
    fn is_transparent(&self) -> bool {
        match self {
            BlockType::GLASS => true,
            _ => false
        }
    }

    fn is_passthrough(&self) -> bool {
        match self {
            _ => false
        }
    }
}

#[derive(Debug)]
pub struct Chunk {
    pub layers: [[BlockType; CHUNK_AREA]; 2],
    pub light: [u8; CHUNK_AREA]
}

#[derive(Component)]
pub struct ChunkComponent {
    pub position: IVec2
}

#[derive(Component)]
pub struct ChunkLayer;

#[derive(Event, Debug, Clone, Copy)]
pub struct PlaceBlock
{
    pub layer: PlaceMode,
    pub position: UVec2,
    pub id: u8,
    pub entity: Entity
}

#[derive(Event)]
pub struct CalcLightChunks;

#[derive(Event)]
pub struct RemeshChunks;

#[derive(Event)]
pub struct RecollisionChunk {
    pub entity: Entity
}

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
   fn build(&self, app: &mut App) {
        app.add_event::<PlaceBlock>();
        app.add_event::<RemeshChunks>();
        app.add_event::<CalcLightChunks>();
        app.add_event::<RecollisionChunk>();
        app.add_systems(Update, (calculate_lighting, remesh, regenerate_collision).chain().in_set(GameSystemSet::Chunk));
   } 
}

fn calculate_lighting(
    mut chunks: ResMut<Chunks>,
    mut calc_light_ev: EventReader<CalcLightChunks>,
) {
    for _ in calc_light_ev.read() {
        // Iterate more times so it propagates
        for _ in 0..16 {
            // First pass: collect the light data
            let mut light_updates = Vec::new();
            for (chunk_pos, chunk) in chunks.iter() {
                let mut light = [0; CHUNK_AREA];
                for i in 0..CHUNK_AREA {
                    if chunk.layers[PlaceMode::BLOCK as usize][i] <= BlockType::AIR && chunk.layers[PlaceMode::WALL as usize][i] <= BlockType::AIR || 
                        chunk.layers[PlaceMode::BLOCK as usize][i].is_transparent() || chunk.layers[PlaceMode::WALL as usize][i].is_transparent()
                    {
                        light[i] = 15;
                    } else {
                        let pos = get_position_from_index(i);
                        let global = get_global_position(*chunk_pos, pos);
                        if let Some(neighbors) = get_neighboring_lights(&chunks, global) {
                            let max = neighbors.iter().max().unwrap();
                            if *max > 0 {
                                light[i] = max.saturating_sub(1);
                            }
                        }
                    }
                }
                light_updates.push((*chunk_pos, light));
            }
    
            // Second pass: apply the light updates
            for (chunk_pos, light) in light_updates {
                if let Some(m_chunk) = chunks.get_mut(&chunk_pos) {
                    // Apply the light updates to m_chunk here
                    m_chunk.light = light;
                }
            }
        }
    }
}


fn remesh(
    mut remesh_chunk_ev: EventReader<RemeshChunks>,
    chunks: Res<Chunks>,
    chunk_query: Query<(&Children, &ChunkComponent)>,
    chunk_layer_query: Query<&Mesh2dHandle, With<ChunkLayer>>,
    mut meshes: ResMut<Assets<Mesh>>
) {
    for _ in remesh_chunk_ev.read() {
        for (chunk_children, chunk_comp) in chunk_query.iter() {
            let chunk = chunks.get(&chunk_comp.position).unwrap();
    
            for li in 0..chunk.layers.len() {
                if let Ok(layer_mesh) = chunk_layer_query.get(chunk_children[li]) {
                    let mesh = meshes.get_mut(layer_mesh.0.id()).unwrap();
    
                    let mut vertex_positions = vec![[0.0; 3]; CHUNK_MESH_SIZE];
                    
                    let mut vertex_colors = vec![[0.0; 4]; CHUNK_MESH_SIZE];
                    
                    let mut vertex_uvs = vec![[0.0; 2]; CHUNK_MESH_SIZE];
    
                    let indices: Vec<u32> = generate_chunk_indices();
        
                    for i in 0..CHUNK_AREA {
                        let position = get_position_from_index(i);
            
                        if chunk.layers[li][i] > BlockType::AIR {
                            // Positions
                            vertex_positions[i * VERTICES_PER_BLOCK    ] = [position.x as f32 * TILE_SIZE as f32,                    position.y as f32 * TILE_SIZE as f32,                    0.0];
                            vertex_positions[i * VERTICES_PER_BLOCK + 1] = [position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, position.y as f32 * TILE_SIZE as f32,                    0.0];
                            vertex_positions[i * VERTICES_PER_BLOCK + 2] = [position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, 0.0];
                            vertex_positions[i * VERTICES_PER_BLOCK + 3] = [position.x as f32 * TILE_SIZE as f32,                    position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, 0.0];
        
                            // Vertex Colors
                            let light = chunk.light[i] as f32 / 15.0;
                            let mut color: Color = Color::rgb(light, light, light);
                            if li == PlaceMode::WALL as usize { color = Color::GRAY * light; }
                            vertex_colors[i * VERTICES_PER_BLOCK    ] = [color.r(), color.g(), color.b(), 1.0];
                            vertex_colors[i * VERTICES_PER_BLOCK + 1] = [color.r(), color.g(), color.b(), 1.0];
                            vertex_colors[i * VERTICES_PER_BLOCK + 2] = [color.r(), color.g(), color.b(), 1.0];
                            vertex_colors[i * VERTICES_PER_BLOCK + 3] = [color.r(), color.g(), color.b(), 1.0];
        
                            // Set block UVs
                            vertex_uvs[i * VERTICES_PER_BLOCK    ] = [(1.0 / (BlockType::SIZE as usize - 1) as f32) * (chunk.layers[li][i] as usize - 1) as f32, 1.0];
                            vertex_uvs[i * VERTICES_PER_BLOCK + 1] = [(1.0 / (BlockType::SIZE as usize - 1) as f32) *  chunk.layers[li][i] as usize      as f32, 1.0];
                            vertex_uvs[i * VERTICES_PER_BLOCK + 2] = [(1.0 / (BlockType::SIZE as usize - 1) as f32) *  chunk.layers[li][i] as usize      as f32, 0.0];
                            vertex_uvs[i * VERTICES_PER_BLOCK + 3] = [(1.0 / (BlockType::SIZE as usize - 1) as f32) * (chunk.layers[li][i] as usize - 1) as f32, 0.0];
    
                        }
                    }
        
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_positions);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vertex_uvs);
                    mesh.insert_indices(Indices::U32(indices));
                }
            }
        }
    }
}

fn regenerate_collision(
    mut commands: Commands,
    chunks: Res<Chunks>,
    mut recol_chunk_ev: EventReader<RecollisionChunk>,
    chunk_query: Query<(&Children, &ChunkComponent)>,
    collider_query: Query<&Transform, With<Collider>>
) {
    for ev in recol_chunk_ev.read() {
        let (children, chunk_compo) = chunk_query.get(ev.entity).unwrap();
        let chunk = chunks.get(&chunk_compo.position).unwrap();

        // First check if there are colliders for blocks that don't exist anymore
        for c in children.iter() {
            if let Ok(collider_transform) = collider_query.get(*c) {
                let pos = (collider_transform.translation.xy() / TILE_SIZE as f32) - 0.5;
                let index = get_index_from_position(UVec2::new(pos.x as u32, pos.y as u32));
                if chunk.layers[PlaceMode::BLOCK as usize][index] <= BlockType::AIR {
                    commands.entity(*c).despawn_recursive();
                }
            }
        }

        // Now check if there are blocks that does not have a collider yet
        for i in 0..CHUNK_AREA {
            if chunk.layers[PlaceMode::BLOCK as usize][i] > BlockType::AIR {
                let mut has_collider: bool = false;
                for c in children.iter() {
                    if let Ok(collider_transform) = collider_query.get(*c) {
                        let pos = (collider_transform.translation.xy() / TILE_SIZE as f32) - 0.5;
                        let index = get_index_from_position(UVec2::new(pos.x as u32, pos.y as u32));
                        if i == index {
                            has_collider = true;
                            break;
                        }
                    }
                }

                if !has_collider {
                    let pos = get_position_from_index(i);
                    let pixel_pos = Vec2::new(pos.x as f32 * TILE_SIZE as f32, pos.y as f32 * TILE_SIZE as f32);
    
                    commands.spawn(
                        (
                            Name::new(format!("Block Collider at ({}, {})", pos.x, pos.y)),
                            Collider::rectangle(TILE_SIZE as f32, TILE_SIZE as f32),
                            TransformBundle::from_transform(Transform::from_xyz((TILE_SIZE as f32 / 2.0) + pixel_pos.x, (TILE_SIZE as f32 / 2.0) + pixel_pos.y, 0.0))
                        )
                    ).set_parent(ev.entity);
                }
            }
        }
    }
}

pub fn generate_chunk_layer_mesh() -> Mesh
{
    let mut mesh_vec: Vec<[f32; 3]> = Vec::with_capacity(CHUNK_MESH_SIZE);
    for _ in 0..CHUNK_MESH_SIZE { mesh_vec.push([0.0, 0.0, 0.0]); }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_vec)
        .with_inserted_indices(Indices::U32(generate_chunk_indices()))
}

fn generate_chunk_indices() -> Vec<u32>
{
    let mut vec: Vec<u32> = vec![0; CHUNK_INDEX_COUNT];

    let mut offset: usize = 0;
    for i in (0..CHUNK_INDEX_COUNT).step_by(6)
    {
        vec[i + 0] = 0 + offset as u32;
        vec[i + 1] = 1 + offset as u32;
        vec[i + 2] = 2 + offset as u32;

        vec[i + 3] = 2 + offset as u32;
        vec[i + 4] = 3 + offset as u32;
        vec[i + 5] = 0 + offset as u32;

        offset += 4;
    }

    return vec;
}

fn flip_quad(quad_index: usize, indices: &mut Vec<u32>) {
    let i = quad_index * INDICES_PER_BLOCK;
    let offset = quad_index * 4;

    indices[i + 0] = 0 + offset as u32;
    indices[i + 1] = 1 + offset as u32;
    indices[i + 2] = 3 + offset as u32;

    indices[i + 3] = 1 + offset as u32;
    indices[i + 4] = 2 + offset as u32;
    indices[i + 5] = 3 + offset as u32;
}