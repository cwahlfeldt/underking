use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{EnemyKind, GameEntity, Health, HexPosition, Stats},
    grid::{TileData, is_passable, update_ranges},
    hex::{Hex, HexGrid},
};

use super::Enemy;

pub const GRUNT_COUNT: usize = 2;

pub fn spawn_grunts(commands: &mut Commands, grid: &mut HexGrid<TileData>, candidates: &[Hex]) {
    for &start_coord in candidates.iter().take(GRUNT_COUNT) {
        let stats = Stats {
            move_range: 1,
            attack_range: 1,
        };

        let entity = commands
            .spawn((
                Enemy,
                GameEntity,
                EnemyKind::Melee,
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

        update_ranges(grid, start_coord, entity, &stats);
    }
}

/// Melee AI: pathfind toward the player, take up to move_range steps.
pub fn compute_move(
    grid: &HexGrid<TileData>,
    current: Hex,
    player_hex: Hex,
    stats: &Stats,
) -> (Hex, Vec<Hex>) {
    let path = grid.astar(current, player_hex, |h| {
        if h == player_hex {
            return true;
        }
        is_passable(grid, h)
    });

    let mut destination = current;
    let mut move_path = vec![current];

    if let Some(path) = &path {
        let steps = (path.len() - 1).min(stats.move_range as usize);
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
    }

    // Fallback: random traversable neighbor
    if destination == current {
        let mut neighbors: Vec<Hex> = grid
            .neighbors(current)
            .into_iter()
            .filter(|&h| is_passable(grid, h))
            .collect();
        let mut rng = rand::rng();
        neighbors.shuffle(&mut rng);
        if let Some(&fallback) = neighbors.first() {
            destination = fallback;
            move_path.push(fallback);
        }
    }

    (destination, move_path)
}
