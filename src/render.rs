use bevy::prelude::*;

use crate::components::{HexPosition, MovePath};
use crate::hex::HEX_SIZE;

/// Pixels per second for movement along the path.
pub const MOVE_SPEED: f32 = 220.0;
/// Rotation slerp rate — higher = snappier turns.
pub const TURN_SPEED: f32 = 27.0;
/// Easing curve applied to movement progress (0..1 → 0..1).
/// Swap to any of: ease_linear, ease_in_quad, ease_out_quad, ease_in_out_quad,
/// ease_in_cubic, ease_out_cubic, ease_in_out_cubic
pub const EASE_FN: fn(f32) -> f32 = ease_out_cubic;

// --- Easing functions (swap EASE_FN above to use a different one) ---
#[allow(dead_code)]
pub fn ease_linear(t: f32) -> f32 {
    t
}
#[allow(dead_code)]
pub fn ease_in_quad(t: f32) -> f32 {
    t * t
}
#[allow(dead_code)]
pub fn ease_out_quad(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}
#[allow(dead_code)]
pub fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}
#[allow(dead_code)]
pub fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}
#[allow(dead_code)]
pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
#[allow(dead_code)]
pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (animate_movement, sync_hex_to_transform).chain(),
        );
    }
}

fn sync_hex_to_transform(
    mut query: Query<
        (&HexPosition, &mut Transform),
        (
            Or<(Changed<HexPosition>, Added<Transform>)>,
            Without<MovePath>,
        ),
    >,
) {
    for (hex_pos, mut transform) in &mut query {
        let (x, y) = hex_pos.0.to_pixel(HEX_SIZE);
        transform.translation.x = x;
        transform.translation.y = y;
    }
}

fn animate_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut MovePath)>,
) {
    for (entity, mut transform, mut path) in &mut query {
        let from = path.waypoints[path.current_index];
        let to = path.waypoints[path.current_index + 1];

        let distance = from.distance(to);
        if distance > 0.0 {
            path.progress += time.delta_secs() * path.speed / distance;
        } else {
            path.progress = 1.0;
        }

        if path.progress >= 1.0 {
            path.current_index += 1;
            path.progress = 0.0;

            // Snap to the waypoint we just arrived at
            let arrived = path.waypoints[path.current_index];
            transform.translation.x = arrived.x;
            transform.translation.y = arrived.y;

            // If this was the last segment, remove the component
            if path.current_index + 1 >= path.waypoints.len() {
                commands.entity(entity).remove::<MovePath>();
                continue;
            }
        } else {
            let t = EASE_FN(path.progress);
            let pos = from.lerp(to, t);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }

        // Smoothly rotate to face movement direction
        let from = path.waypoints[path.current_index];
        let next = path.waypoints[(path.current_index + 1).min(path.waypoints.len() - 1)];
        let dir = next - from;
        if dir.length_squared() > 0.0 {
            let target_angle = dir.y.atan2(dir.x);
            let target_rot = Quat::from_rotation_z(target_angle);
            let turn_speed = TURN_SPEED;
            transform.rotation = transform
                .rotation
                .slerp(target_rot, (turn_speed * time.delta_secs()).min(1.0));
        }
    }
}

pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    if hex.len() != 7 || !hex.starts_with('#') {
        panic!("Invalid hex color: {hex}. Expected format: #RRGGBB");
    }

    let hex = hex.trim_start_matches("#");
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.0;

    (r, g, b)
}
