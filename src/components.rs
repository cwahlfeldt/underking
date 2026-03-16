use bevy::prelude::*;

use crate::hex::Hex;

/// Marker for all game entities that should be despawned on reset.
#[derive(Component)]
pub struct GameEntity;

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

/// Marker: this enemy attacked the player after the player moved,
/// so it should skip its movement this turn (Rule 4).
#[derive(Component)]
pub struct SkipTurn;

/// Marker: entity is dead. Hidden visually, cleared from grid occupancy/ranges,
/// but not despawned so undo can restore it.
#[derive(Component)]
pub struct Dead;

/// Z-offset added on top of isometric depth sorting.
/// Entities get 1.0 to render above tiles.
#[derive(Component)]
pub struct ZOffset(pub f32);

/// Like MovePath but for undo/redo visual transitions.
/// Doesn't interact with turn logic (check_animation_done ignores it).
#[derive(Component)]
pub struct RewindPath {
    pub from: Vec2,
    pub to: Vec2,
    pub progress: f32,
    pub speed: f32,
}

/// Drives a melee attack animation: lunge toward target, then return.
#[derive(Component)]
pub struct AttackAnimation {
    /// Pixel position of the attacker's home hex.
    pub home: Vec2,
    /// Pixel position of the target enemy's hex (lunge destination).
    pub target: Vec2,
    /// Entity being attacked (to kill when animation finishes).
    pub target_entity: Entity,
    /// Hex of the target (to clear from grid).
    pub target_hex: crate::hex::Hex,
    /// 0.0..1.0 progress through the current phase.
    pub progress: f32,
    /// Speed in pixels/sec.
    pub speed: f32,
    /// Current phase of the attack.
    pub phase: AttackPhase,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttackPhase {
    /// Lunging toward the enemy.
    LungeForward,
    /// Returning to home position.
    LungeBack,
}
