use bevy::prelude::*;

use crate::{
    components::{Bomb, GameEntity, HexPosition, Stats, ZOffset},
    grid::TileData,
    hex::{Hex, HexGrid, HEX_SIZE},
};

use super::{pathfind_and_step, random_passable_neighbor};

/// Min/max throw distance (in hex tiles).
const THROW_MIN: i32 = 2;
const THROW_MAX: i32 = 3;
const BOMB_BLAST_RADIUS: i32 = 1;
const BOMB_DAMAGE: f32 = 1.0;
const BOMB_FUSE_TURNS: u8 = 2;

/// Bomber AI result — either move toward the player or throw a bomb.
pub enum BomberAction {
    Move(Hex, Vec<Hex>),
    ThrowBomb { target_hex: Hex },
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
            return BomberAction::ThrowBomb { target_hex: target };
        }
    }

    // Otherwise move toward the player, but stop at throw range
    let (destination, move_path) =
        pathfind_and_step(grid, current, player_hex, stats.move_range);

    if destination != current && destination.distance(player_hex) >= THROW_MIN {
        return BomberAction::Move(destination, move_path);
    }

    // Fallback: random traversable neighbor
    if let Some(fallback) = random_passable_neighbor(grid, current, |_| true) {
        return BomberAction::Move(fallback, vec![current, fallback]);
    }

    BomberAction::Move(current, vec![current])
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
