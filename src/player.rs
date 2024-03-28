use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy_xpbd_2d::{components::{LinearVelocity, Position, RigidBody, Rotation}, math::Vector, plugins::{collision::{Collider, Collisions}, spatial_query::{ShapeCaster, ShapeHits}}, SubstepSchedule, SubstepSet};

use crate::{chunk::TILE_SIZE, GameState};
use crate::utils::lerp;

const PLAYER_SIZE: f32 = 28.0;
const GRAVITY_ACCEL: f32 = 98.07;
const TERMINAL_GRAVITY: f32 = 530.0;

#[derive(Component)]
pub struct Player {
    pub is_on_ground: bool,
    pub direction: i8
}

#[derive(Component)]
struct PlayerSprite
{
    pub rotation: f32
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Game), spawn_player);
        app.add_systems(Update, 
            (
                player_input,
                apply_gravity,
                update_grounded,
                rotate_player
            ).chain().run_if(in_state(GameState::Game))
        );
        app.add_systems(
            SubstepSchedule,
            solve_collisions.in_set(SubstepSet::SolveUserConstraints).run_if(in_state(GameState::Game)),
        );
    }
}

fn spawn_player(
    mut commands: Commands
) {
    let player_collider = Collider::rectangle(PLAYER_SIZE, PLAYER_SIZE);

    commands.spawn(
        (
            Name::new("Player"),
            RigidBody::Kinematic,
            player_collider.clone(),
            ShapeCaster::new(player_collider, Vector::ZERO, 0.0, Direction2d::NEG_Y).with_max_time_of_impact(0.625),
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(PLAYER_SIZE)),
                    color: Color::rgba(1.0, 1.0, 1.0, 0.0),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 2.0),
                ..default()
            },
            Player {is_on_ground: false, direction: 0 }
        )
    ).with_children(|parent| {
        parent.spawn(
            (
                Name::new("Player Sprite"),
                SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(PLAYER_SIZE)),
                        color: Color::RED,
                        ..default()
                    },
                    ..default()
                },
                PlayerSprite {rotation: 0.0}
            )
        );
    });
}

fn update_grounded(
    mut player_query: Query<(&ShapeHits, &mut Player)>
) {
    for (hits, mut player) in player_query.iter_mut() {
        player.is_on_ground = hits.iter().any(|hit| {
            if hit.normal1.y > 0.0 || hit.normal2.y > 0.0 {
                true
            } else {
                false
            }
        });
    }
}

fn rotate_player(
    player_query: Query<&Player>,
    mut player_sprite_query: Query<(&mut Transform, &mut PlayerSprite)>,
    time: Res<Time>
) {
    if let Ok((mut sprite_transform, mut player_sprite)) = player_sprite_query.get_single_mut() {
        if let Ok(player) = player_query.get_single() {
            if !player.is_on_ground {
                player_sprite.rotation -= (9.6 * time.delta_seconds()) * player.direction as f32;
            }

            if player.direction == 0 {
                let nineties = (player_sprite.rotation / FRAC_PI_2).round() * FRAC_PI_2;
                player_sprite.rotation = lerp(player_sprite.rotation, nineties, 0.25);
            }
            
            sprite_transform.rotation = Quat::from_axis_angle(Vec3::Z, player_sprite.rotation);
        }
    }
}

fn solve_collisions(
    collisions: Res<Collisions>,
    mut player_query: Query<(&mut Position, &mut LinearVelocity), With<Player>>
) {
    for contacts in collisions.iter() {
        if !contacts.during_current_substep {
            continue;
        }

        let is_first: bool;
        let (mut position, mut linear_velocity) = 
            if let Ok(player) = player_query.get_mut(contacts.entity1) {
                is_first = true;
                player
            } else if let Ok(player) = player_query.get_mut(contacts.entity2) {
                is_first = false;
                player
            } else {
                continue;
            };

        for manifold in contacts.manifolds.iter() {
            let normal = if is_first {
                -manifold.global_normal1(&Rotation::ZERO)
            } else {
                -manifold.global_normal2(&Rotation::ZERO)
            };

            for contact in manifold.contacts.iter().filter(|c| c.penetration > 0.0) {
                position.0 += normal * contact.penetration;
                if normal.y != 0.0 {
                    linear_velocity.y = 0.0;
                }
                if normal.x != 0.0 {
                    linear_velocity.x = 0.0;
                }
            }
        }
    }
}

fn apply_gravity(
    mut player_query: Query<(&mut LinearVelocity, &Player)>,
    time: Res<Time>
) {
    if let Ok((mut player_velocity, player)) = player_query.get_single_mut() {
        if !player.is_on_ground {
            if player_velocity.y > -TERMINAL_GRAVITY {
                player_velocity.y -= (GRAVITY_ACCEL * TILE_SIZE as f32) * time.delta_seconds();
            } else if player_velocity.y < -TERMINAL_GRAVITY {
                player_velocity.y = -TERMINAL_GRAVITY;
            }
        }
    }
}

fn player_input(
    mut player_query: Query<(&mut LinearVelocity, &mut Player)>,
    keyboard_input: Res<ButtonInput<KeyCode>>
) {
    if let Ok((mut player_linear_velocity, mut player)) = player_query.get_single_mut() {
        let speed: f32 = TILE_SIZE as f32 * 10.0;
        let jump_force = 16.0 * TILE_SIZE as f32;
    
        if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
            player_linear_velocity.0.x = lerp(player_linear_velocity.0.x, -speed, 0.25);
            player.direction = -1;
        }
        else if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
            player_linear_velocity.0.x = lerp(player_linear_velocity.0.x, speed, 0.25);
            player.direction = 1;
        } else {
            player_linear_velocity.0.x = lerp(player_linear_velocity.0.x, 0.0, 0.25);
            if player.is_on_ground {
                player.direction = 0;
            }
        }

        // if keyboard_input.pressed(KeyCode::Up) || keyboard_input.pressed(KeyCode::W) {
        //     player_linear_velocity.0.y = speed;
        // } else if keyboard_input.pressed(KeyCode::Down) || keyboard_input.pressed(KeyCode::S) {
        //     player_linear_velocity.0.y = -speed;
        // } else {
        //     player_linear_velocity.0.y = 0.0;
        // }

        if keyboard_input.pressed(KeyCode::Space) || keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
           if player.is_on_ground {
               player_linear_velocity.0.y = jump_force;
           }
        }
    }
}