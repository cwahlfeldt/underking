use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    Turn, TurnPhase, TurnState,
    components::{Dead, HexPosition, MovePath, SkipTurn, Stats},
    entities::player::Player,
    grid::{TileData, clear_ranges, is_passable, update_ranges},
    hex::{HEX_SIZE, Hex, HexGrid},
    render::MOVE_SPEED,
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

#[derive(Resource, Default)]
pub struct EnemyTurnQueue(pub Vec<Entity>);

const ENEMY_COUNT: usize = 2;
const MIN_SPAWN_DISTANCE: i32 = 4;

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

    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| h.distance(player_hex) >= MIN_SPAWN_DISTANCE && is_passable(&grid, h))
        .collect();
    candidates.shuffle(&mut rng);

    let spawn_count = ENEMY_COUNT.min(candidates.len());

    let stats = Stats {
        move_range: 1,
        attack_range: 1,
    };

    for &start_coord in candidates.iter().take(spawn_count) {
        let entity = commands
            .spawn((
                Enemy,
                crate::components::Health {
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

        update_ranges(&mut grid, start_coord, entity, &stats);
    }

    commands.insert_resource(EnemyTurnQueue::default());
}

fn render_enemy(mut commands: Commands, query: Query<Entity, Added<Enemy>>) {
    for entity in &query {
        commands.entity(entity).insert((
            Transform::default(),
            Text2d::new("@"),
            TextColor(Color::srgb(0.776, 0.259, 0.922)),
        ));
    }
}

fn move_enemies(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut queue: ResMut<EnemyTurnQueue>,
    mut move_order: ResMut<crate::undo::TurnMoveOrder>,
    animating: Query<(), With<MovePath>>,
    player_query: Query<&HexPosition, With<Player>>,
    mut enemy_query: Query<(Entity, &mut HexPosition, &Stats, Option<&SkipTurn>), (With<Enemy>, Without<Player>, Without<Dead>)>,
) {
    if *turn != TurnState::Active(Turn::Enemy) {
        return;
    }

    if !animating.is_empty() {
        return;
    }

    if queue.0.is_empty() {
        queue.0 = enemy_query.iter().map(|(e, _, _, _)| e).collect();
    }

    let Some(enemy_entity) = queue.0.pop() else {
        *turn = TurnState::Active(Turn::Player);
        return;
    };

    let Ok(player_pos) = player_query.single() else {
        *turn = TurnState::Active(Turn::Player);
        return;
    };
    let player_hex = player_pos.0;

    let Ok((entity, mut hex_pos, stats, skip)) = enemy_query.get_mut(enemy_entity) else {
        // Entity may have been despawned (killed) - skip it
        return;
    };

    // Rule 4: enemies that already attacked the player skip their movement.
    if skip.is_some() {
        commands.entity(entity).remove::<SkipTurn>();
        // Check if queue is empty to advance turn
        if queue.0.is_empty() {
            *turn = TurnState::Active(Turn::Player);
        }
        return;
    }

    let current = hex_pos.0;

    // Pathfind toward the player (allow player's tile as goal)
    let path = grid.astar(current, player_hex, |h| {
        if h == player_hex {
            return true;
        }
        is_passable(&grid, h)
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
            .filter(|&h| is_passable(&grid, h))
            .collect();
        let mut rng = rand::rng();
        neighbors.shuffle(&mut rng);
        if let Some(&fallback) = neighbors.first() {
            destination = fallback;
            move_path.push(fallback);
        }
    }

    if destination != current {
        // Record this enemy's move in the turn order
        move_order.0.push(entity);

        // Update grid state directly (can't use move_entity for enemy mid-queue)
        let old_pos = hex_pos.0;
        hex_pos.0 = destination;

        if let Some(tile) = grid.get_mut(old_pos) {
            tile.occupant = None;
        }
        if let Some(tile) = grid.get_mut(destination) {
            tile.occupant = Some(entity);
        }

        clear_ranges(&mut grid, entity);
        update_ranges(&mut grid, destination, entity, stats);

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

    if queue.0.is_empty() {
        if destination != current {
            *turn = TurnState::Animating { next: TurnPhase::Turn(Turn::Player) };
        } else {
            *turn = TurnState::Active(Turn::Player);
        }
    }
}
