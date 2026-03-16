use bevy::prelude::*;

use crate::{
    components::{AttackAnimation, AttackPhase, Dead, Health, HexPosition, SkipTurn, Stats},
    entities::{enemy::Enemy, player::Player},
    grid::{self, TileData},
    hex::{HexGrid, HEX_SIZE},
    render::MOVE_SPEED,
    turn::{CombatPhase, GameSettings, PendingKills, Turn, TurnState},
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
    mut pending_kills: ResMut<PendingKills>,
    mut player_query: Query<(Entity, &HexPosition, &Stats, &mut Health), With<Player>>,
    enemy_query: Query<(Entity, &HexPosition), (With<Enemy>, Without<Player>, Without<Dead>)>,
    attack_query: Query<(), (With<Player>, With<AttackAnimation>)>,
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
            // at the player's PREVIOUS position.
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

            if !killed_enemies.is_empty() {
                // Start attack animation toward the first enemy, queue all kills
                let (_, first_enemy_hex) = killed_enemies[0];
                let (px, py) = player_hex.to_iso_pixel(HEX_SIZE);
                let (ex, ey) = first_enemy_hex.to_iso_pixel(HEX_SIZE);

                commands.entity(player_entity).insert(AttackAnimation {
                    home: Vec2::new(px, py),
                    target: Vec2::new(ex, ey),
                    target_entity: killed_enemies[0].0,
                    target_hex: first_enemy_hex,
                    progress: 0.0,
                    speed: MOVE_SPEED * 1.8,
                    phase: AttackPhase::LungeForward,
                });

                pending_kills.0 = killed_enemies;
                *turn = TurnState::Combat(CombatPhase::PlayerAttackAnimating);
            } else {
                // No kills — check if enemies attack the player
                apply_enemy_attacks(
                    &mut commands,
                    &grid,
                    player_entity,
                    player_hex,
                    &mut player_health,
                );
                *turn = TurnState::Active(Turn::Enemy);
            }
        }
        CombatPhase::PlayerAttackAnimating => {
            // Wait for attack animation to finish
            if !attack_query.is_empty() {
                return;
            }

            // Animation done — apply all pending kills
            let Ok((player_entity, player_pos, _player_stats, mut player_health)) =
                player_query.single_mut()
            else {
                pending_kills.0.clear();
                *turn = TurnState::Active(Turn::Enemy);
                return;
            };

            for (enemy_entity, enemy_hex) in pending_kills.0.drain(..) {
                info!("Player kills enemy {:?} at {:?}", enemy_entity, enemy_hex);
                if let Some(tile) = grid.get_mut(enemy_hex) {
                    tile.occupant = None;
                }
                grid::clear_ranges(&mut grid, enemy_entity);
                commands
                    .entity(enemy_entity)
                    .insert((Dead, Visibility::Hidden));
            }

            // Now check if enemies attack the player
            let player_hex = player_pos.0;
            apply_enemy_attacks(
                &mut commands,
                &grid,
                player_entity,
                player_hex,
                &mut player_health,
            );

            *turn = TurnState::Active(Turn::Enemy);
        }
        CombatPhase::AfterEnemyMove => {
            *turn = TurnState::Active(Turn::Player);
        }
    }
}

fn apply_enemy_attacks(
    commands: &mut Commands,
    grid: &HexGrid<TileData>,
    player_entity: Entity,
    player_hex: crate::hex::Hex,
    player_health: &mut Health,
) {
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
        }
    }
}
