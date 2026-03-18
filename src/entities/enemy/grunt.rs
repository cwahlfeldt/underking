use crate::{
    components::Stats,
    grid::TileData,
    hex::{Hex, HexGrid},
};

use super::{pathfind_and_step, random_passable_neighbor};

/// Melee AI: pathfind toward the player, take up to move_range steps.
/// Falls back to a random passable neighbor if stuck.
pub fn compute_move(
    grid: &HexGrid<TileData>,
    current: Hex,
    player_hex: Hex,
    stats: &Stats,
) -> (Hex, Vec<Hex>) {
    let (destination, move_path) = pathfind_and_step(grid, current, player_hex, stats.move_range);

    if destination != current {
        return (destination, move_path);
    }

    // Fallback: random traversable neighbor
    if let Some(fallback) = random_passable_neighbor(grid, current, |_| true) {
        return (fallback, vec![current, fallback]);
    }

    (current, vec![current])
}
