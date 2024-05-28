use bevy::math::*;
use crate::chunk::*;

pub fn lerp(a: f32, b: f32, f: f32) -> f32
{
    return a * (1.0 - f) + (b * f);
}

pub fn vec3_a_bigger_than_b(a: Vec3, b: Vec3) -> bool {
    return  a.x > b.x &&
            a.y > b.y &&
            a.z > b.z
}

pub fn get_position_from_index(index: usize) -> UVec2 {
    return UVec2::new(
        index as u32 % CHUNK_WIDTH as u32,
        (index as u32 / CHUNK_WIDTH as u32) % CHUNK_WIDTH as u32
    );
}

pub fn get_chunk_diff(relative_pos: IVec2) -> IVec2 {
    return IVec2::new((relative_pos.x as f32 / CHUNK_WIDTH as f32).floor() as i32, (relative_pos.y as f32 / CHUNK_WIDTH as f32).floor() as i32);
}

// pub fn get_global_position(relative_pos: IVec2, chunk_position: IVec2) -> IVec2 {
//     let chunk_diff = get_chunk_diff(relative_pos);
    
//     let mut true_chunk_pos = chunk_position - chunk_diff;

//     let fixed_pos = IVec2::new(
//         modular(relative_pos.x, CHUNK_WIDTH as i32),
//         modular(relative_pos.y, CHUNK_WIDTH as i32)
//     );

//     true_chunk_pos *= CHUNK_WIDTH as i32;

//     return true_chunk_pos + fixed_pos;
// }

pub fn relative_coord_is_inside_bounds(coord: IVec2) -> bool {
    return  coord.x >= 0 && coord.x < CHUNK_WIDTH as i32 &&
            coord.y >= 0 && coord.y < CHUNK_WIDTH as i32
}

pub fn get_index_from_position(position: UVec2) -> usize {
    return position.x as usize + (position.y as usize * CHUNK_WIDTH);
}

pub fn get_block_position(pixel_position: Vec2) -> IVec2 {
    return IVec2::new(
        (pixel_position.x / TILE_SIZE as f32).floor() as i32,
        (pixel_position.y / TILE_SIZE as f32).floor() as i32,
    );
}

pub fn get_chunk_position(block_position: IVec2) -> IVec2 {
    return IVec2::new(
        (block_position.x as f32 / CHUNK_WIDTH as f32).floor() as i32,
        (block_position.y as f32 / CHUNK_WIDTH as f32).floor() as i32
    );
}

pub fn get_relative_position(global_position: IVec2, chunk_position: IVec2) -> UVec2 {
    return UVec2::new(
        (global_position.x as f32 - (chunk_position.x as f32 * CHUNK_WIDTH as f32)) as u32,
        (global_position.y as f32 - (chunk_position.y as f32 * CHUNK_WIDTH as f32)) as u32
    );
}

pub fn get_chunk_position_from_translation(translation: Vec2) -> IVec2 {
    return IVec2::new(
        (translation.x as i32 / CHUNK_WIDTH as i32) / TILE_SIZE as i32,
        (translation.y as i32 / CHUNK_WIDTH as i32) / TILE_SIZE as i32
    );
}

pub fn get_global_position(chunk_position: IVec2, relative_pos: UVec2) -> IVec2 {
    let to_block = IVec2::new(chunk_position.x * CHUNK_WIDTH as i32, chunk_position.y * CHUNK_WIDTH as i32);
    return IVec2::new(to_block.x + relative_pos.x as i32, to_block.y + relative_pos.y as i32);
}

pub fn modular(a: i32, b: i32) -> i32
{
    return ((a % b) + b) % b;
}