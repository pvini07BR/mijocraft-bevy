pub const TILE_SIZE: usize = 32;

pub const CHUNK_WIDTH: usize = 16;
pub const CHUNK_AREA: usize = CHUNK_WIDTH*CHUNK_WIDTH;

const VERTICES_PER_BLOCK: usize = 4;
const INDICES_PER_BLOCK: usize = 6;

const CHUNK_MESH_SIZE: usize = CHUNK_AREA * VERTICES_PER_BLOCK;
const CHUNK_INDEX_COUNT: usize = CHUNK_AREA * INDICES_PER_BLOCK;

pub const AVAILABLE_BLOCKS: usize = 5;

use bevy::{math::Vec3A, prelude::*, render::{mesh::{Indices, PrimitiveTopology}, primitives::Aabb, render_asset::RenderAssetUsages}, sprite::{Anchor, MaterialMesh2dBundle, Mesh2dHandle}};
use bevy_xpbd_2d::{components::RigidBody, plugins::collision::Collider};

use crate::{chunk_manager::{get_block, GetBlockSysParam}, utils::{get_index_from_position, get_position_from_index}, world::GameSystemSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaceMode {
    WALL = 0,
    BLOCK = 1
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Chunk {
    pub position: IVec2
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ChunkLayer
{
    pub blocks: [u8; CHUNK_AREA],
    pub color: Color
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct BlockChunkLayer;

#[derive(Event, Debug, Clone, Copy)]
pub struct PlaceBlock
{
    pub layer: PlaceMode,
    pub position: UVec2,
    pub id: u8,
    pub entity: Entity
}

#[derive(Event)]
pub struct SpawnChunk {
    pub position: IVec2,
    pub blocks: [u8; CHUNK_AREA],
    pub walls: [u8; CHUNK_AREA]
}

#[derive(Event)]
pub struct RemeshChunk {
    pub entity: Entity
}

#[derive(Event)]
pub struct RecollisionChunk {
    pub entity: Entity
}

pub struct ChunkPlugin;
impl Plugin for ChunkPlugin {
   fn build(&self, app: &mut App) {
        app.add_event::<PlaceBlock>();
        app.add_event::<SpawnChunk>();
        app.add_event::<RemeshChunk>();
        app.add_event::<RecollisionChunk>();
        app.register_type::<ChunkLayer>();
        app.register_type::<Chunk>();
        app.register_type::<BlockChunkLayer>();
        app.add_systems(Update, (spawn_chunk, set_block, remesh, regenerate_collision).chain().in_set(GameSystemSet::Chunk));
   } 
}

pub fn spawn_chunk(
    mut commands: Commands, 
    mut spawn_chunk_ev: EventReader<SpawnChunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    chunk_query: Query<&Transform, With<Chunk>>,
    asset_server: Res<AssetServer>,
    mut remesh_chunk_ev: EventWriter<RemeshChunk>,
    mut recol_chunk_ev: EventWriter<RecollisionChunk>
) {
    for ev in spawn_chunk_ev.read() {
        let pixel_chunk_pos = Vec2::new((ev.position.x as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32, (ev.position.y as f32 * CHUNK_WIDTH as f32) * TILE_SIZE as f32);

        for c_transform in chunk_query.iter() {
            if c_transform.translation.xy() == pixel_chunk_pos { return; }
        }

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
                Chunk {position: ev.position},
            )
        ).with_children(|parent| {
            parent.spawn((
                Name::new("Chunk Wall Layer"),
                ChunkLayer { blocks: ev.walls, color: Color::rgba(0.25, 0.25, 0.25, 1.0) },
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
            ));

            parent.spawn((
                Name::new("Chunk Block Layer"),
                ChunkLayer { blocks: ev.blocks, color: Color::WHITE },
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
                BlockChunkLayer
            ));
        }).id();

        remesh_chunk_ev.send(RemeshChunk { entity: id });
        recol_chunk_ev.send(RecollisionChunk { entity: id });
    }
}

fn set_block(
    mut chunk_query: Query<(&Children, Entity), With<Chunk>>,
    mut chunk_layer_query: Query<&mut ChunkLayer>,
    mut place_block_ev: EventReader<PlaceBlock>,
    mut remesh_chunk_ev: EventWriter<RemeshChunk>,
    mut recol_chunk_ev: EventWriter<RecollisionChunk>
) {
    for ev in place_block_ev.read()
    {
        if let Ok((children, _)) = chunk_query.get_mut(ev.entity) {
            chunk_layer_query.get_mut(children[ev.layer as usize]).unwrap().blocks[get_index_from_position(ev.position)] = ev.id;

            for (_, entity) in chunk_query.iter() {
                remesh_chunk_ev.send(RemeshChunk { entity });
                recol_chunk_ev.send(RecollisionChunk { entity });
            }
        }
    }
}

fn remesh(
    mut remesh_chunk_ev: EventReader<RemeshChunk>,
    chunk_query: Query<(&Children, &Transform), With<Chunk>>,
    chunk_layer_query: Query<(&ChunkLayer, &Mesh2dHandle)>,
    mut sys_param: GetBlockSysParam<'_, '_>,
    mut meshes: ResMut<Assets<Mesh>>,
    block_layer_q: Query<&ChunkLayer, With<BlockChunkLayer>>
) {
    for ev in remesh_chunk_ev.read() {
        let (chunk_children, chunk_transform) = chunk_query.get(ev.entity).unwrap();

        for ci in 0..chunk_children.len() {
            if let Ok((chunk_layer, layer_mesh)) = chunk_layer_query.get(chunk_children[ci]) {
                let mesh = meshes.get_mut(layer_mesh.0.id()).unwrap();

                let mut vertex_positions: Vec<[f32; 3]> = Vec::with_capacity(CHUNK_MESH_SIZE);
                for _ in 0..CHUNK_MESH_SIZE { vertex_positions.push([0.0, 0.0, 0.0]); }
            
                let mut vertex_colors: Vec<[f32; 4]> = Vec::with_capacity(CHUNK_MESH_SIZE);
                for _ in 0..CHUNK_MESH_SIZE { vertex_colors.push([0.0, 0.0, 0.0, 0.0]); }
    
                let mut vertex_uvs: Vec<[f32; 2]> = Vec::with_capacity(CHUNK_MESH_SIZE);
                for _ in 0..CHUNK_MESH_SIZE { vertex_uvs.push([0.0, 0.0]); }

                let mut indices: Vec<u32> = generate_chunk_indices();
    
                for i in 0..CHUNK_AREA {
                    let position = get_position_from_index(i);
        
                    if chunk_layer.blocks[i] > 0 {
                        // Positions
                        vertex_positions[i * VERTICES_PER_BLOCK    ] = [position.x as f32 * TILE_SIZE as f32,                    position.y as f32 * TILE_SIZE as f32,                    0.0];
                        vertex_positions[i * VERTICES_PER_BLOCK + 1] = [position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, position.y as f32 * TILE_SIZE as f32,                    0.0];
                        vertex_positions[i * VERTICES_PER_BLOCK + 2] = [position.x as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, 0.0];
                        vertex_positions[i * VERTICES_PER_BLOCK + 3] = [position.x as f32 * TILE_SIZE as f32,                    position.y as f32 * TILE_SIZE as f32 + TILE_SIZE as f32, 0.0];
    
                        // Vertex Colors
                        vertex_colors[i * VERTICES_PER_BLOCK    ] = chunk_layer.color.as_rgba_f32();
                        vertex_colors[i * VERTICES_PER_BLOCK + 1] = chunk_layer.color.as_rgba_f32();
                        vertex_colors[i * VERTICES_PER_BLOCK + 2] = chunk_layer.color.as_rgba_f32();
                        vertex_colors[i * VERTICES_PER_BLOCK + 3] = chunk_layer.color.as_rgba_f32();
    
                        // Set block UVs
                        vertex_uvs[i * VERTICES_PER_BLOCK    ] = [(1.0 / AVAILABLE_BLOCKS as f32) * (chunk_layer.blocks[i] - 1) as f32, 1.0];
                        vertex_uvs[i * VERTICES_PER_BLOCK + 1] = [(1.0 / AVAILABLE_BLOCKS as f32) *  chunk_layer.blocks[i] as f32,       1.0];
                        vertex_uvs[i * VERTICES_PER_BLOCK + 2] = [(1.0 / AVAILABLE_BLOCKS as f32) *  chunk_layer.blocks[i] as f32,       0.0];
                        vertex_uvs[i * VERTICES_PER_BLOCK + 3] = [(1.0 / AVAILABLE_BLOCKS as f32) * (chunk_layer.blocks[i] - 1) as f32, 0.0];

                        // Ambient Occlusion
                        if ci == 0 {
                            if let Ok(block_layer) = block_layer_q.get(chunk_children[PlaceMode::BLOCK as usize]) {
                                const AO_COLOR: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
    
                                let chunk_position = IVec2::new(
                                    (chunk_transform.translation.x as i32 / CHUNK_WIDTH as i32) / TILE_SIZE as i32,
                                    (chunk_transform.translation.y as i32 / CHUNK_WIDTH as i32) / TILE_SIZE as i32
                                );
        
                                let int_pos = IVec2::new(position.x as i32, position.y as i32);
        
                                // Sides

                                // Up
                                let up = get_block(&mut sys_param, int_pos + IVec2::Y, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if up > 0 && up != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 2] = AO_COLOR;
                                    vertex_colors[i * VERTICES_PER_BLOCK + 3] = AO_COLOR;
                                }

                                // Right
                                let right = get_block(&mut sys_param, int_pos + IVec2::X, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if right > 0 && right != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 1] = AO_COLOR;
                                    vertex_colors[i * VERTICES_PER_BLOCK + 2] = AO_COLOR;
                                }

                                // Down
                                let down = get_block(&mut sys_param, int_pos + IVec2::NEG_Y, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if down > 0 && down != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 0] = AO_COLOR;
                                    vertex_colors[i * VERTICES_PER_BLOCK + 1] = AO_COLOR;
                                }

                                // Left
                                let left = get_block(&mut sys_param, int_pos + IVec2::NEG_X, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if left > 0 && left != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 0] = AO_COLOR;
                                    vertex_colors[i * VERTICES_PER_BLOCK + 3] = AO_COLOR;
                                }

                                // Corners

                                // Bottom Left
                                let bl = get_block(&mut sys_param, int_pos + IVec2::NEG_ONE, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if bl > 0 && bl != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 0] = AO_COLOR;
                                    flip_quad(i, &mut indices);
                                }

                                // Bottom Right
                                let br = get_block(&mut sys_param, int_pos + IVec2::new(1, -1), chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if br > 0 && br != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 1] = AO_COLOR;
                                }

                                // Top Right
                                let tr = get_block(&mut sys_param, int_pos + IVec2::ONE, chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if tr > 0 && tr != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 2] = AO_COLOR;
                                    flip_quad(i, &mut indices);
                                }

                                // Top Left
                                let tl = get_block(&mut sys_param, int_pos + IVec2::new(-1, 1), chunk_position, PlaceMode::BLOCK, &block_layer.blocks);
                                if tl > 0 && tl != 5 {
                                    vertex_colors[i * VERTICES_PER_BLOCK + 3] = AO_COLOR;
                                }
                            }
                        }
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

fn regenerate_collision(
    mut commands: Commands,
    mut recol_chunk_ev: EventReader<RecollisionChunk>,
    chunk_query: Query<&Children, With<Chunk>>,
    chunk_layer_query: Query<&ChunkLayer>,
    collider_query: Query<Entity, With<Collider>>
) {
    for ev in recol_chunk_ev.read() {
        let children = chunk_query.get(ev.entity).unwrap();
        for c in children.iter() {
            if collider_query.contains(*c) {
                commands.entity(*c).despawn_recursive();
            }
        }
        let block_chunk_layer = chunk_layer_query.get(children[PlaceMode::BLOCK as usize]).unwrap();

        for i in 0..CHUNK_AREA {
            if block_chunk_layer.blocks[i] > 0 {
                let pos = get_position_from_index(i);
                let pixel_pos = Vec2::new(pos.x as f32 * TILE_SIZE as f32, pos.y as f32 * TILE_SIZE as f32);

                commands.spawn(
                    (
                        Name::new("Chunk collider shape"),
                        Collider::rectangle(TILE_SIZE as f32, TILE_SIZE as f32),
                        TransformBundle::from_transform(Transform::from_xyz((TILE_SIZE as f32 / 2.0) + pixel_pos.x, (TILE_SIZE as f32 / 2.0) + pixel_pos.y, 0.0))
                    )
                ).set_parent(ev.entity);
            }
        }
    }
}

fn generate_chunk_layer_mesh() -> Mesh
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