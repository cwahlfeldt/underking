use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{AttackPattern, EnemyKind, GameEntity, Health, HexPosition, Stats},
    grid::{TileData, is_passable, update_ranges_with_pattern},
    hex::{Hex, HexGrid},
};

use super::Enemy;

pub const WARLOCK_COUNT: usize = 1;

pub const ATTACK_PATTERN: AttackPattern = AttackPattern::AllDirectionsRanged {
    min_range: 2,
    max_range: 5,
};

pub fn spawn_warlocks(commands: &mut Commands, grid: &mut HexGrid<TileData>, candidates: &[Hex]) {
    for &start_coord in candidates.iter().take(WARLOCK_COUNT) {
        let stats = Stats {
            move_range: 1,
            attack_range: 5,
        };

        let entity = commands
            .spawn((
                Enemy,
                GameEntity,
                EnemyKind::Warlock,
                ATTACK_PATTERN,
                Health {
                    current: 1.0,
                    max: 1.0,
                },
                HexPosition(start_coord),
                Stats {
                    move_range: stats.move_range,
                    attack_range: stats.attack_range,
                },
            ))
            .id();

        if let Some(tile) = grid.get_mut(start_coord) {
            tile.occupant = Some(entity);
        }

        update_ranges_with_pattern(grid, start_coord, entity, &stats, Some(&ATTACK_PATTERN));
    }
}

/// Warlock AI: find a hex along any of the 6 directions from the player at distance 2-4,
/// then pathfind there. If already in a valid position, stay put.
pub fn compute_move(
    grid: &HexGrid<TileData>,
    current: Hex,
    player_hex: Hex,
    stats: &Stats,
) -> (Hex, Vec<Hex>) {
    // Check if already in a valid attack position
    let valid_positions = player_hex.all_directions_attack_hexes(2, 4);
    if valid_positions.contains(&current) {
        return (current, vec![current]);
    }

    // Sort candidates by distance from current position (prefer closer ones)
    let mut candidates: Vec<(Hex, i32)> = valid_positions
        .into_iter()
        .filter(|&h| grid.contains(h) && (h == current || is_passable(grid, h)))
        .map(|h| (h, current.distance(h)))
        .collect();
    candidates.sort_by_key(|&(_, d)| d);

    // Try pathfinding to each candidate, closest first
    for (target, _) in &candidates {
        let path = grid.astar(current, *target, |h| {
            if h == *target {
                return true;
            }
            is_passable(grid, h)
        });

        if let Some(path) = path {
            let steps = (path.len() - 1).min(stats.move_range as usize);
            let mut destination = current;
            let mut move_path = vec![current];

            for i in 1..=steps {
                let candidate = path[i];
                let occupied = grid
                    .get(candidate)
                    .map(|t| t.occupant.is_some())
                    .unwrap_or(true);
                if occupied {
                    break;
                }
                destination = candidate;
                move_path.push(candidate);
            }

            if destination != current {
                return (destination, move_path);
            }
        }
    }

    // Fallback: move away from the player if too close
    if current.distance(player_hex) <= 1 {
        let mut neighbors: Vec<Hex> = grid
            .neighbors(current)
            .into_iter()
            .filter(|&h| is_passable(grid, h) && h.distance(player_hex) > 1)
            .collect();
        let mut rng = rand::rng();
        neighbors.shuffle(&mut rng);
        if let Some(&fallback) = neighbors.first() {
            return (fallback, vec![current, fallback]);
        }
    }

    (current, vec![current])
}
