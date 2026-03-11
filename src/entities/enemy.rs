use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    Turn, TurnState,
    components::{Health, HexPosition, MovePath},
    entities::player::{Player, clear_ranges, update_ranges},
    hex::{HEX_SIZE, Hex, HexGrid, TileData},
    render::{MOVE_SPEED, hex_to_rgb},
};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            spawn_enemies
                .after(super::tile::spawn_tiles)
                .after(super::player::spawn_player),
        )
        .add_systems(Update, (render_enemy, move_enemies).chain());
    }
}

#[derive(Component)]
pub struct Enemy;

/// Tracks which enemies still need to move this turn.
#[derive(Resource, Default)]
pub struct EnemyTurnQueue(pub Vec<Entity>);

const ENEMY_COUNT: usize = 3;
const MOVE_RANGE: i32 = 1;
const ATTACK_RANGE: i32 = 1;
const MIN_SPAWN_DISTANCE: i32 = 3;

pub fn spawn_enemies(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    player_query: Query<&HexPosition, With<Player>>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    let player_hex = player_pos.0;

    let mut rng = rand::rng();

    // Collect traversable, unoccupied tiles at least MIN_SPAWN_DISTANCE from the player
    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| {
            h.distance(player_hex) >= MIN_SPAWN_DISTANCE
                && grid
                    .get(h)
                    .map(|t| t.traversable && t.occupant.is_none())
                    .unwrap_or(false)
        })
        .collect();
    candidates.shuffle(&mut rng);

    let spawn_count = ENEMY_COUNT.min(candidates.len());

    for &start_coord in candidates.iter().take(spawn_count) {
        let entity = commands
            .spawn((
                Enemy,
                Health {
                    current: 3.0,
                    max: 3.0,
                },
                HexPosition(start_coord),
            ))
            .id();

        if let Some(tile) = grid.get_mut(start_coord) {
            tile.occupant = Some(entity);
        }

        let attack_hexes: Vec<Hex> = start_coord
            .spiral(ATTACK_RANGE)
            .into_iter()
            .filter(|&h| h != start_coord && grid.contains(h))
            .collect();

        for hex in attack_hexes {
            if let Some(tile) = grid.get_mut(hex) {
                tile.attack_ranges.push(entity);
            }
        }
    }

    commands.insert_resource(EnemyTurnQueue::default());
}

fn render_enemy(mut commands: Commands, query: Query<Entity, Added<Enemy>>) {
    let (r, g, b) = hex_to_rgb("#c642eb");
    for entity in &query {
        commands.entity(entity).insert((
            Transform::default(),
            Text2d::new("@"),
            TextColor(Color::srgb(r, g, b)),
        ));
    }
}

fn move_enemies(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut queue: ResMut<EnemyTurnQueue>,
    animating: Query<(), With<MovePath>>,
    player_query: Query<&HexPosition, With<Player>>,
    mut enemy_query: Query<(Entity, &mut HexPosition), (With<Enemy>, Without<Player>)>,
) {
    // Only act on enemy's turn
    if *turn != TurnState::Active(Turn::Enemy) {
        return;
    }

    // Wait for any ongoing animation to finish before moving next enemy
    if !animating.is_empty() {
        return;
    }

    // Build the queue on first call of this turn
    if queue.0.is_empty() {
        queue.0 = enemy_query.iter().map(|(e, _)| e).collect();
    }

    // Pop next enemy from the queue
    let Some(enemy_entity) = queue.0.pop() else {
        // All enemies have moved — back to player
        *turn = TurnState::Active(Turn::Player);
        return;
    };

    let Ok(player_pos) = player_query.single() else {
        *turn = TurnState::Active(Turn::Player);
        return;
    };
    let player_hex = player_pos.0;

    let Ok((entity, mut hex_pos)) = enemy_query.get_mut(enemy_entity) else {
        // Entity gone, move on to next
        return;
    };

    let current = hex_pos.0;

    // Try to pathfind toward the player
    let path = grid.astar(current, player_hex, |h| {
        if h == player_hex {
            return true;
        }
        grid.get(h)
            .map(|t| t.traversable && t.occupant.is_none())
            .unwrap_or(false)
    });

    let mut destination = current;
    let mut move_path = vec![current];

    if let Some(path) = &path {
        let steps = (path.len() - 1).min(MOVE_RANGE as usize);
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

    // Fallback: if can't move toward player, pick a random traversable neighbor
    if destination == current {
        let mut neighbors: Vec<Hex> = grid
            .neighbors(current)
            .into_iter()
            .filter(|&h| {
                grid.get(h)
                    .map(|t| t.traversable && t.occupant.is_none())
                    .unwrap_or(false)
            })
            .collect();
        let mut rng = rand::rng();
        neighbors.shuffle(&mut rng);
        if let Some(&fallback) = neighbors.first() {
            destination = fallback;
            move_path.push(fallback);
        }
    }

    if destination != current {
        hex_pos.0 = destination;

        if let Some(tile) = grid.get_mut(current) {
            tile.occupant = None;
        }
        if let Some(tile) = grid.get_mut(destination) {
            tile.occupant = Some(entity);
        }

        clear_ranges(&mut grid, entity);
        update_ranges(&mut grid, destination, entity);

        let waypoints: Vec<Vec2> = move_path
            .iter()
            .map(|h| {
                let (x, y) = h.to_pixel(HEX_SIZE);
                Vec2::new(x, y)
            })
            .collect();

        commands.entity(entity).insert(MovePath {
            waypoints,
            current_index: 0,
            progress: 0.0,
            speed: MOVE_SPEED,
        });
    }

    // If there are more enemies in the queue, stay on enemy turn (animation check will gate next move)
    // If queue is empty, transition to player turn
    if queue.0.is_empty() {
        if destination != current {
            // Last enemy moved with animation — wait for it
            *turn = TurnState::Animating {
                next: Turn::Player,
            };
        } else {
            *turn = TurnState::Active(Turn::Player);
        }
    }
    // Otherwise stay in Active(Enemy) — next frame will wait for animation then move next enemy
}
