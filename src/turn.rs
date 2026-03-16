use bevy::prelude::*;

use crate::hex::Hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Player,
    Enemy,
}

/// Phases inserted between movement and next turn for combat resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatPhase {
    /// After player moves: player attacks enemies in range, then enemies attack player if in range.
    AfterPlayerMove,
    /// Waiting for player attack animation to finish before applying kills.
    PlayerAttackAnimating,
    /// After all enemies move: (reserved for future use).
    AfterEnemyMove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum TurnState {
    /// Waiting for this turn's entity to act.
    Active(Turn),
    /// An entity is animating; once done, switch to `next`.
    Animating { next: TurnPhase },
    /// Resolving combat after movement.
    Combat(CombatPhase),
}

/// What to transition to after an animation finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnPhase {
    Turn(Turn),
    Combat(CombatPhase),
}

#[derive(Resource)]
pub struct GameSettings {
    pub selected_hex: Option<Hex>,
    pub hovered_enemy: Option<Entity>,
    /// Currently hovered bomb entity (to show blast radius).
    pub hovered_bomb: Option<Entity>,
    /// Player's hex before the current move, used by combat resolution.
    pub player_prev_hex: Option<Hex>,
}

/// Enemies queued to be killed after the player's attack animation finishes.
#[derive(Resource, Default)]
pub struct PendingKills(pub Vec<(Entity, Hex)>);
