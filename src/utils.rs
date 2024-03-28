use bevy::math::Vec3;

pub fn lerp(a: f32, b: f32, f: f32) -> f32
{
    return a * (1.0 - f) + (b * f);
}

pub fn vec3_a_bigger_than_b(a: Vec3, b: Vec3) -> bool {
    return  a.x > b.x &&
            a.y > b.y &&
            a.z > b.z
}