use bevy::prelude::*;

use crate::components::{HexPosition, MovePath, RewindPath, ZOffset};
use crate::entities::player::FacingDirection;
use crate::hex::{HEX_SIZE, iso_z, iso_z_from_y};

/// Pixels per second for movement along the path.
pub const MOVE_SPEED: f32 = 150.0;
/// Rotation slerp rate — higher = snappier turns.
pub const TURN_SPEED: f32 = 27.0;
/// Easing curve applied to movement progress (0..1 → 0..1).
/// Swap to any of: ease_linear, ease_in_quad, ease_out_quad, ease_in_out_quad,
/// ease_in_cubic, ease_out_cubic, ease_in_out_cubic
pub const EASE_FN: fn(f32) -> f32 = ease_linear;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easing_boundaries() {
        let fns: &[(&str, fn(f32) -> f32)] = &[
            ("linear", ease_linear),
            ("in_quad", ease_in_quad),
            ("out_quad", ease_out_quad),
            ("in_out_quad", ease_in_out_quad),
            ("in_cubic", ease_in_cubic),
            ("out_cubic", ease_out_cubic),
            ("in_out_cubic", ease_in_out_cubic),
        ];

        for (name, f) in fns {
            let at_0 = f(0.0);
            let at_1 = f(1.0);
            assert!(at_0.abs() < 1e-6, "{name}: f(0) = {at_0}, expected 0");
            assert!(
                (at_1 - 1.0).abs() < 1e-6,
                "{name}: f(1) = {at_1}, expected 1"
            );
        }
    }

    #[test]
    fn easing_monotonic() {
        let fns: &[fn(f32) -> f32] = &[
            ease_linear,
            ease_in_quad,
            ease_out_quad,
            ease_in_out_quad,
            ease_in_cubic,
            ease_out_cubic,
            ease_in_out_cubic,
        ];

        for f in fns {
            let mut prev = f(0.0);
            for i in 1..=100 {
                let t = i as f32 / 100.0;
                let val = f(t);
                assert!(val >= prev - 1e-6, "easing not monotonic at t={t}");
                prev = val;
            }
        }
    }

    #[test]
    fn ease_in_out_symmetric() {
        for f in [ease_in_out_quad, ease_in_out_cubic] {
            let a = f(0.25);
            let b = f(0.75);
            assert!(
                (a + b - 1.0).abs() < 1e-6,
                "in_out not symmetric: f(0.25)={a}, f(0.75)={b}"
            );
        }
    }
}

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (animate_movement, animate_rewind, sync_hex_to_transform).chain(),
        );
    }
}

fn sync_hex_to_transform(
    mut query: Query<
        (&HexPosition, &mut Transform, Option<&ZOffset>),
        (
            Or<(Changed<HexPosition>, Added<Transform>)>,
            Without<MovePath>,
            Without<RewindPath>,
        ),
    >,
) {
    for (hex_pos, mut transform, z_offset) in &mut query {
        let (x, y) = hex_pos.0.to_iso_pixel(HEX_SIZE);
        transform.translation.x = x;
        transform.translation.y = y;
        transform.translation.z = iso_z(&hex_pos.0) + z_offset.map(|z| z.0).unwrap_or(0.0);
    }
}

fn animate_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut MovePath,
        Option<&FacingDirection>,
        Option<&ZOffset>,
    )>,
) {
    for (entity, mut transform, mut path, has_facing, z_offset) in &mut query {
        let z_off = z_offset.map(|z| z.0).unwrap_or(0.0);
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
            transform.translation.z = iso_z_from_y(arrived.y) + z_off;

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
            transform.translation.z = iso_z_from_y(pos.y) + z_off;
        }

        // Smoothly rotate to face movement direction (skip for sprite-based entities)
        if has_facing.is_none() {
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
}

fn animate_rewind(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Transform,
        &mut RewindPath,
        Option<&ZOffset>,
        Option<&FacingDirection>,
    )>,
) {
    for (entity, mut transform, mut rewind, z_offset, has_facing) in &mut query {
        let z_off = z_offset.map(|z| z.0).unwrap_or(0.0);
        // speed == 0 means "hold in place" — don't advance or remove
        if rewind.speed == 0.0 {
            continue;
        }

        let distance = rewind.from.distance(rewind.to);
        if distance > 0.0 {
            rewind.progress += time.delta_secs() * rewind.speed / distance;
        } else {
            rewind.progress = 1.0;
        }

        if rewind.progress >= 1.0 {
            transform.translation.x = rewind.to.x;
            transform.translation.y = rewind.to.y;
            transform.translation.z = iso_z_from_y(rewind.to.y) + z_off;
            commands.entity(entity).remove::<RewindPath>();
        } else {
            let t = ease_in_cubic(rewind.progress);
            let pos = rewind.from.lerp(rewind.to, t);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            transform.translation.z = iso_z_from_y(pos.y) + z_off;
        }

        // Smoothly rotate to face movement direction (skip for sprite-based entities)
        if has_facing.is_none() {
            let dir = rewind.to - rewind.from;
            if dir.length_squared() > 0.0 {
                let target_angle = dir.y.atan2(dir.x);
                let target_rot = Quat::from_rotation_z(target_angle);
                transform.rotation = transform
                    .rotation
                    .slerp(target_rot, (TURN_SPEED * time.delta_secs()).min(1.0));
            }
        }
    }
}
