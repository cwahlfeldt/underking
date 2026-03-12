use bevy::prelude::*;

use crate::{
    components::{Dead, Health, HexPosition, SkipTurn, Stats},
    entities::{enemy::Enemy, player::Player},
    grid::{self, TileData},
    hex::HexGrid,
    turn::{CombatPhase, GameSettings, Turn, TurnState},
};

/// Resolve combat after player movement.
///
/// Rule 6: If player moved to a tile within attack range of an enemy -> kill that enemy.
/// Rule 4: If player is on a tile within an enemy's attack range -> enemy attacks player,
///         and that enemy skips its movement turn.
pub fn resolve_combat(
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
                let adjacent_now = player_hex.distance(enemy_hex) <= player_stats.attack_range;
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
