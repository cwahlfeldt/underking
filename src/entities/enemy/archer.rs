use crate::{
    components::{AttackPattern, Stats},
    grid::{TileData, is_passable},
    hex::{Hex, HexGrid},
};

use super::{pathfind_and_step, random_passable_neighbor};

pub const ATTACK_PATTERN: AttackPattern = AttackPattern::DiagonalRanged {
    min_range: 2,
    max_range: 4,
};

/// Archer AI: find a hex on the player's diagonals (NE-SW or NW-SE) at distance 2-4,
/// then pathfind there. If already in a valid position, stay put.
pub fn compute_move(
    grid: &HexGrid<TileData>,
    current: Hex,
    player_hex: Hex,
    stats: &Stats,
) -> (Hex, Vec<Hex>) {
    // Check if already in a valid attack position
    let valid_positions = player_hex.diagonal_attack_hexes(2, 4);
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
        let (destination, move_path) = pathfind_and_step(grid, current, *target, stats.move_range);
        if destination != current {
            return (destination, move_path);
        }
    }

    // Fallback: move away from the player if too close
    if current.distance(player_hex) <= 1 {
        if let Some(fallback) =
            random_passable_neighbor(grid, current, |h| h.distance(player_hex) > 1)
        {
            return (fallback, vec![current, fallback]);
        }
    }

    (current, vec![current])
}
