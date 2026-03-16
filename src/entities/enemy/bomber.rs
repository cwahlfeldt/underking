use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{Bomb, EnemyKind, GameEntity, Health, HexPosition, Stats, ZOffset},
    grid::{TileData, is_passable, update_ranges},
    hex::{Hex, HexGrid, HEX_SIZE},
};

use super::Enemy;

pub const BOMBER_COUNT: usize = 1;

/// Min/max throw distance (in hex tiles).
const THROW_MIN: i32 = 2;
const THROW_MAX: i32 = 3;
const BOMB_BLAST_RADIUS: i32 = 1;
const BOMB_DAMAGE: f32 = 1.0;
const BOMB_FUSE_TURNS: u8 = 2;

pub fn spawn_bombers(commands: &mut Commands, grid: &mut HexGrid<TileData>, candidates: &[Hex]) {
    for &start_coord in candidates.iter().take(BOMBER_COUNT) {
        let stats = Stats {
            move_range: 1,
            attack_range: 0, // bomber doesn't use attack_ranges on tiles
        };

        let entity = commands
            .spawn((
                Enemy,
                GameEntity,
                EnemyKind::Bomber,
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

/// Bomber AI result — either move toward the player or throw a bomb.
pub enum BomberAction {
    Move(Hex, Vec<Hex>),
    ThrowBomb { bomber_hex: Hex, target_hex: Hex },
}

/// Bomber AI: if within throw range (2-3 tiles) of the player, throw a bomb.
/// Otherwise, move toward the player like a melee grunt.
pub fn compute_action(
    grid: &HexGrid<TileData>,
    current: Hex,
    player_hex: Hex,
    stats: &Stats,
) -> BomberAction {
    let dist = current.distance(player_hex);

    // If in throw range, throw bomb onto a neighboring tile of the player
    // (within bomber's range, not on the player itself)
    if dist >= THROW_MIN && dist <= THROW_MAX {
        let mut best_target = None;
        let mut best_dist = i32::MAX;
        for neighbor in player_hex.neighbors() {
            if grid.contains(neighbor) && neighbor != current {
                let d = current.distance(neighbor);
                if d >= THROW_MIN && d <= THROW_MAX && d < best_dist {
                    best_dist = d;
                    best_target = Some(neighbor);
                }
            }
        }
        // Fallback: any neighbor on the grid within range
        if best_target.is_none() {
            for neighbor in player_hex.neighbors() {
                if grid.contains(neighbor) && neighbor != current {
                    best_target = Some(neighbor);
                    break;
                }
            }
        }
        if let Some(target) = best_target {
            return BomberAction::ThrowBomb {
                bomber_hex: current,
                target_hex: target,
            };
        }
    }

    // Otherwise move toward the player, but stop at throw range
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
            // Don't move closer than throw range
            if candidate.distance(player_hex) < THROW_MIN {
                break;
            }
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

    BomberAction::Move(destination, move_path)
}

/// Spawn a bomb entity on the given hex, owned by `owner`.
pub fn spawn_bomb(commands: &mut Commands, owner: Entity, target_hex: Hex) {
    let (x, y) = target_hex.to_iso_pixel(HEX_SIZE);
    commands.spawn((
        GameEntity,
        Bomb {
            owner,
            turns_remaining: BOMB_FUSE_TURNS,
            blast_radius: BOMB_BLAST_RADIUS,
            damage: BOMB_DAMAGE,
        },
        HexPosition(target_hex),
        Transform::from_xyz(x, y, 0.5),
        Text2d::new("*"),
        TextFont {
            font_size: 32.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.0)),
        ZOffset(0.5),
    ));
}
