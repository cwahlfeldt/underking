use bevy::prelude::*;

use crate::hex::Hex;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexPosition(pub Hex);

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
