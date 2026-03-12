mod components;
mod debug_ui;
mod entities;
mod grid;
mod hex;
mod render;
mod undo;

use crate::components::{Dead, Health, HexPosition, MovePath, SkipTurn, Stats};
use crate::entities::enemy::Enemy;
use crate::entities::player::Player;
use crate::grid::TileData;
use crate::hex::{Hex, HexGrid};
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub selected_hex: Option<Hex>,
    pub hovered_enemy: Option<Entity>,
    /// Player's hex before the current move, used by combat resolution.
    pub player_prev_hex: Option<Hex>,
}

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

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: None,
            hovered_enemy: None,
            player_prev_hex: None,
        })
        .insert_resource(TurnState::Active(Turn::Player))
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            // FpsOverlayPlugin::default(),
        ))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_plugins(debug_ui::DebugUiPlugin)
        .add_plugins(undo::UndoPlugin)
        .add_systems(Update, (check_animation_done, resolve_combat).chain())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// When all MovePath animations finish, advance to the next phase.
fn check_animation_done(mut turn: ResMut<TurnState>, animating: Query<(), With<MovePath>>) {
    let TurnState::Animating { next } = *turn else {
        return;
    };

    if animating.is_empty() {
        *turn = match next {
            TurnPhase::Turn(t) => TurnState::Active(t),
            TurnPhase::Combat(c) => TurnState::Combat(c),
        };
    }
}

/// Resolve combat after player movement.
///
/// Rule 6: If player moved to a tile within attack range of an enemy → kill that enemy.
/// Rule 4: If player is on a tile within an enemy's attack range → enemy attacks player,
///         and that enemy skips its movement turn.
fn resolve_combat(
    mut commands: Commands,
    mut turn: ResMut<TurnState>,
    mut grid: ResMut<HexGrid<TileData>>,
    game_settings: Res<GameSettings>,
    mut player_query: Query<(Entity, &HexPosition, &Stats, &mut Health), With<Player>>,
    enemy_query: Query<(Entity, &HexPosition), (With<Enemy>, Without<Player>, Without<Dead>)>,
) {
    let TurnState::Combat(phase) = *turn else {
        return;
    };

    match phase {
        CombatPhase::AfterPlayerMove => {
            let Ok((player_entity, player_pos, player_stats, mut player_health)) =
                player_query.single_mut()
            else {
                *turn = TurnState::Active(Turn::Enemy);
                return;
            };
            let player_hex = player_pos.0;
            let prev_hex = game_settings.player_prev_hex;

            // Rule 6: Player attacks by moving to a tile adjacent to an enemy,
            // but only if that enemy was already within the player's attack range
            // at the player's PREVIOUS position (i.e. the enemy walked into range).
            let mut killed_enemies = Vec::new();
            for (enemy_entity, enemy_pos) in &enemy_query {
                let enemy_hex = enemy_pos.0;
                // Enemy must be adjacent to the player's new position
                let adjacent_now = player_hex.distance(enemy_hex) <= player_stats.attack_range;
                // Enemy must have been in the player's attack range at the old position
                let was_in_range = prev_hex
                    .map(|prev| prev.distance(enemy_hex) <= player_stats.attack_range)
                    .unwrap_or(false);
                if adjacent_now && was_in_range {
                    killed_enemies.push((enemy_entity, enemy_hex));
                }
            }

            for (enemy_entity, enemy_hex) in &killed_enemies {
                info!("Player kills enemy {:?} at {:?}", enemy_entity, enemy_hex);
                if let Some(tile) = grid.get_mut(*enemy_hex) {
                    tile.occupant = None;
                }
                grid::clear_ranges(&mut grid, *enemy_entity);
                commands
                    .entity(*enemy_entity)
                    .insert((Dead, Visibility::Hidden));
            }

            // Rule 4: Enemies whose attack range covers the player's tile attack the player.
            // Those enemies skip their movement turn.
            if let Some(player_tile) = grid.get(player_hex) {
                let attackers: Vec<Entity> = player_tile
                    .attack_ranges
                    .iter()
                    .copied()
                    .filter(|&e| e != player_entity)
                    .collect();

                for attacker in &attackers {
                    info!("Enemy {:?} attacks player!", attacker);
                    player_health.current -= 1.0;
                    commands.entity(*attacker).insert(SkipTurn);
                }
                if player_health.current <= 0.0 {
                    info!("Player died!");
                    // TODO: handle game over
                }
            }

            *turn = TurnState::Active(Turn::Enemy);
        }
        CombatPhase::AfterEnemyMove => {
            // Rule 5: Enemies that moved into player's attack range are NOT attacked yet.
            // Simply advance to player's turn.
            *turn = TurnState::Active(Turn::Player);
        }
    }
}
