pub const TILE_SIZE: usize = 32;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_AREA: usize = CHUNK_WIDTH * CHUNK_WIDTH;

const VERTICES_PER_BLOCK: usize = 4;
const INDICES_PER_BLOCK: usize = 6;

const CHUNK_MESH_SIZE: usize = CHUNK_AREA * VERTICES_PER_BLOCK;
const CHUNK_INDEX_COUNT: usize = CHUNK_AREA * INDICES_PER_BLOCK;

use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
    sprite::Mesh2dHandle,
};
use bevy_xpbd_2d::prelude::*;
use enum_iterator::Sequence;

use crate::{
    chunk_manager::Chunks, utils::{
        get_global_position, get_index_from_position, get_neighboring_blocks, get_neighboring_blocks_with_corners, get_neighboring_lights, get_neighboring_lights_with_corners, get_position_from_index
    }, GameSettings, GameState
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaceMode {
    WALL = 0,
    BLOCK = 1,
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, PartialOrd, Serialize, Deserialize, Sequence)]
pub enum BlockType {
    #[default]
    AIR = 0,
    GRASS,
    DIRT,
    STONE,
    PLANKS,
    GLASS,
    SIZE,
}

impl BlockType {
    fn is_transparent(&self) -> bool {
        match self {
            BlockType::AIR => true,
            BlockType::GLASS => true,
            _ => false,
        }
    }

    fn is_passthrough(&self) -> bool {
        match self {
            BlockType::AIR => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub layers: [[BlockType; CHUNK_AREA]; 2],
    pub light: [u8; CHUNK_AREA],
}

#[derive(Component)]
pub struct ChunkComponent {
    pub position: IVec2,
}

#[derive(Component)]
pub struct ChunkLayer;

#[derive(Event)]
pub struct CalcLightChunks;

#[derive(Event)]
pub struct RemeshChunks;

#[derive(Event)]
pub struct RecollisionChunk {
    pub entity: Entity,
}

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RemeshChunks>();
        app.add_event::<CalcLightChunks>();
        app.add_event::<RecollisionChunk>();
        app.add_systems(
            Update,
            (calculate_lighting, remesh, regenerate_collision)
                .chain()
                .run_if(in_state(GameState::Game)),
        );
    }
}

fn calculate_lighting(mut chunks: ResMut<Chunks>, mut calc_light_ev: EventReader<CalcLightChunks>) {
    for _ in calc_light_ev.read() {
        // Iterate more times so it propagates
        for _ in 0..16 {
            // First pass: collect the light data
            let mut light_updates = Vec::new();
            for (chunk_pos, chunk) in chunks.iter() {
                let mut light = [0; CHUNK_AREA];
                for i in 0..CHUNK_AREA {
                    if chunk.layers[PlaceMode::BLOCK as usize][i].is_transparent()
                        && chunk.layers[PlaceMode::WALL as usize][i].is_transparent()
                    {
                        light[i] = 15;
                    } else {
                        let pos = get_position_from_index(i);
                        let global = get_global_position(*chunk_pos, pos);
                        if let Some(neighbors) = get_neighboring_lights(&chunks, global) {
                            if let Some(max) = neighbors.iter().max() {
                                if *max > 0 {
                                    light[i] = max.saturating_sub(1);
                                }
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
    mut meshes: ResMut<Assets<Mesh>>,
    settings_res: Res<GameSettings>
) {
    for _ in remesh_chunk_ev.read() {
        for (chunk_children, chunk_comp) in chunk_query.iter() {
            let Some(chunk) = chunks.get(&chunk_comp.position) else {
                continue;
            };

            for li in 0..chunk.layers.len() {
                let Ok(layer_mesh) = chunk_layer_query.get(chunk_children[li]) else {
                    continue;
                };
                let Some(mesh) = meshes.get_mut(layer_mesh.0.id()) else {
                    continue;
                };
                let mut vertex_positions = vec![[0.0; 3]; CHUNK_MESH_SIZE];
                let mut vertex_colors = vec![[0.0; 4]; CHUNK_MESH_SIZE];
                let mut vertex_uvs = vec![[0.0; 2]; CHUNK_MESH_SIZE];
                let mut indices: Vec<u32> = generate_chunk_indices();

                for i in 0..CHUNK_AREA {
                    let position = get_position_from_index(i);
                    if chunk.layers[li][i] <= BlockType::AIR {
                        continue;
                    }

                    // Positions
                    let pos_template = |pos: u32, x: bool| {
                        pos as f32 * TILE_SIZE as f32 + (x as usize * TILE_SIZE) as f32
                    };
                    let p = |a: bool, b: bool| {
                        [
                            pos_template(position.x, a),
                            pos_template(position.y, b),
                            0.0,
                        ]
                    };
                    let vertex_positions = &mut vertex_positions[i * VERTICES_PER_BLOCK..];
                    vertex_positions[0] = p(false, false);
                    vertex_positions[1] = p(true, false);
                    vertex_positions[2] = p(true, true);
                    vertex_positions[3] = p(false, true);

                    // Vertex Colors
                    // ...and also smooth lighting.
                    let wall_darkness = settings_res.wall_darkness;
                    let light = chunk.light[i] as f32 / 15.0;
    
                    let color = match li == PlaceMode::WALL as usize {
                        false => Color::srgb(light, light, light),
                        true => Color::srgb(wall_darkness * light, wall_darkness * light, wall_darkness * light),
                    };

                    for vertex_color in vertex_colors[i * VERTICES_PER_BLOCK..].iter_mut().take(4) {
                        *vertex_color = color.to_linear().to_vec4().to_array();
                    }

                    if settings_res.smooth_lighting {
                        let global = get_global_position(chunk_comp.position, position);
                        if let Some(neighbors) = get_neighboring_lights_with_corners(&chunks, global) {
                            let get_color = |f_light: f32| -> [f32; 4] {
                                if li == PlaceMode::BLOCK as usize {
                                    return [f_light, f_light, f_light, 1.0];
                                } else {
                                    return [wall_darkness * f_light, wall_darkness * f_light, wall_darkness * f_light, 1.0];
                                }
                            };

                            let normalize_light = |light: u8| { return light as f32 / 15.0; };

                            // Bottom Left vertex
                            let average = (
                                normalize_light(neighbors[0]) + // Center
                                normalize_light(neighbors[4]) + // Left
                                normalize_light(neighbors[5]) + // Bottom Left
                                normalize_light(neighbors[1])   // Down
                            ) / 4.0;
                            vertex_colors[i * VERTICES_PER_BLOCK + 0] = get_color(average);

                            // Bottom Right vertex
                            let average = (
                                normalize_light(neighbors[0]) + // Center
                                normalize_light(neighbors[2]) + // Right
                                normalize_light(neighbors[6]) + // Bottom Right
                                normalize_light(neighbors[1])   // Down
                            ) / 4.0;
                            vertex_colors[i * VERTICES_PER_BLOCK + 1] = get_color(average);

                            // Top Right vertex
                            let average = (
                                normalize_light(neighbors[0]) + // Center
                                normalize_light(neighbors[2]) + // Right
                                normalize_light(neighbors[7]) + // Top Right
                                normalize_light(neighbors[3])   // Up
                            ) / 4.0;
                            vertex_colors[i * VERTICES_PER_BLOCK + 2] = get_color(average);

                            // Top Left vertex
                            let average = (
                                normalize_light(neighbors[0]) + // Center
                                normalize_light(neighbors[4]) + // Left
                                normalize_light(neighbors[8]) + // Top Left
                                normalize_light(neighbors[3])   // Up
                            ) / 4.0;
                            vertex_colors[i * VERTICES_PER_BLOCK + 3] = get_color(average);
                        }
                    }

                    // Wall Ambient Occlusion
                    if settings_res.wall_ambient_occlusion && li == PlaceMode::WALL as usize {
                        let global = get_global_position(chunk_comp.position, position);
                        if let Some(neighbors) = get_neighboring_blocks_with_corners(&chunks, global, PlaceMode::BLOCK) {
                            let ao_color: [f32; 4] = [0.1 * light, 0.1 * light, 0.1 * light, 1.0];

                            // Down
                            if !neighbors[1].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 0] = ao_color;
                                vertex_colors[i * VERTICES_PER_BLOCK + 1] = ao_color;
                            }

                            // Right
                            if !neighbors[2].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 1] = ao_color;
                                vertex_colors[i * VERTICES_PER_BLOCK + 2] = ao_color;
                            }

                            // Up
                            if !neighbors[3].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 2] = ao_color;
                                vertex_colors[i * VERTICES_PER_BLOCK + 3] = ao_color;
                            }

                            // Left
                            if !neighbors[4].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 0] = ao_color;
                                vertex_colors[i * VERTICES_PER_BLOCK + 3] = ao_color;
                            }

                            // Now check for the corners!!
                            // ===========================

                            // Bottom Left
                            if !neighbors[5].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 0] = ao_color;
                                flip_quad(i, &mut indices);
                            }

                            // Bottom Right
                            if !neighbors[6].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 1] = ao_color;
                            }

                            // Top Right
                            if !neighbors[7].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 2] = ao_color;
                                flip_quad(i, &mut indices);
                            }

                            // Top Left
                            if !neighbors[8].is_transparent() {
                                vertex_colors[i * VERTICES_PER_BLOCK + 3] = ao_color;
                            }
                        }                        
                    }

                    // Set block UVs
                    let u = |a: i32| {
                        (chunk.layers[li][i] as i32 + a) as f32
                            / (BlockType::SIZE as i32 - 1) as f32
                    };

                    let uvs = &mut vertex_uvs[i * VERTICES_PER_BLOCK..];
                    uvs[0] = [u(-1), 1.0];
                    uvs[1] = [u(0), 1.0];
                    uvs[2] = [u(0), 0.0];
                    uvs[3] = [u(-1), 0.0];
                }

                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_positions);
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vertex_uvs);
                mesh.insert_indices(Indices::U32(indices));
            }
        }
    }
}

fn regenerate_collision(
    mut commands: Commands,
    chunks: Res<Chunks>,
    mut recol_chunk_ev: EventReader<RecollisionChunk>,
    chunk_query: Query<(&Children, &ChunkComponent)>,
    collider_query: Query<&Transform, With<Collider>>,
) {
    let tile_size = TILE_SIZE as f32;
    for ev in recol_chunk_ev.read() {
        let (children, chunk) = {
            let Ok((children, chunk_compo)) = chunk_query.get(ev.entity) else {
                continue;
            };
            let Some(chunk) = chunks.get(&chunk_compo.position) else {
                continue;
            };
            (children, chunk)
        };

        let get_index = |collider: &Transform| {
            let pos = (collider.translation.xy() / tile_size) - 0.5;
            get_index_from_position(pos.as_uvec2())
        };
        let collides =
            |index: usize| chunk.layers[PlaceMode::BLOCK as usize][index] > BlockType::AIR;

        // First check if there are colliders for blocks that don't exist anymore
        for child in children.iter() {
            let Ok(collider) = collider_query.get(*child) else {
                continue;
            };
            if !collides(get_index(collider)) {
                commands.entity(*child).despawn_recursive();
            }
        }

        let has_collider = |index: usize| {
            children
                .iter()
                .filter_map(|child| collider_query.get(*child).ok())
                .any(|collider| get_index(collider) == index)
        };

        // Now check if there are blocks that do not have a collider yet
        for i in (0..CHUNK_AREA)
            .filter(|&i| collides(i))
            .filter(|&i| !has_collider(i))
        {
            let pos = get_position_from_index(i).as_vec2();
            let translation = Vec2::splat(tile_size) * 0.5 + pos * tile_size;
            let transform = Transform::from_translation(translation.extend(0.0));
            let bundle = (
                Name::new(format!("Block Collider at ({}, {})", pos.x, pos.y)),
                Collider::rectangle(tile_size, tile_size),
                TransformBundle::from_transform(transform),
            );
            commands.spawn(bundle).set_parent(ev.entity);
        }
    }
}

pub fn generate_chunk_layer_mesh() -> Mesh {
    let mut mesh_vec: Vec<[f32; 3]> = Vec::with_capacity(CHUNK_MESH_SIZE);
    for _ in 0..CHUNK_MESH_SIZE {
        mesh_vec.push([0.0, 0.0, 0.0]);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, mesh_vec)
    .with_inserted_indices(Indices::U32(generate_chunk_indices()))
}

fn generate_chunk_indices() -> Vec<u32> {
    let mut vec: Vec<u32> = vec![0; CHUNK_INDEX_COUNT];

    let mut offset: usize = 0;
    for i in (0..CHUNK_INDEX_COUNT).step_by(6) {
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
