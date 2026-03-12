use bevy::prelude::*;

use crate::hex::Hex;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexPosition(pub Hex);

/// Per-entity combat stats. Replaces per-module constants.
#[derive(Component)]
pub struct Stats {
    /// 0 means unlimited range (can move to any reachable tile).
    pub move_range: i32,
    pub attack_range: i32,
}

/// Attached to an entity to animate it along a hex path.
/// Waypoints are pixel-space (x, y) positions for each hex in the path.
/// The entity lerps from one waypoint to the next at `speed` pixels/sec.
#[derive(Component)]
pub struct MovePath {
    pub waypoints: Vec<Vec2>,
    pub current_index: usize,
    pub progress: f32,
    pub speed: f32,
}

/// Like MovePath but for undo/redo visual transitions.
/// Doesn't interact with turn logic (check_animation_done ignores it).
#[derive(Component)]
pub struct RewindPath {
    pub from: Vec2,
    pub to: Vec2,
    pub progress: f32,
    pub speed: f32,
}
