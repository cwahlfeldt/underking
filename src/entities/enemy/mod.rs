pub mod archer;
pub mod bomber;
pub mod grunt;
pub mod warlock;

use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{
        AttackPattern, Bomb, Dead, EnemyKind, GameEntity, Health, HexPosition, MovePath, SkipTurn,
        Stats, ZOffset,
    },
    entities::player::Player,
    grid::{self, TileData, is_passable},
    hex::{Hex, HexGrid},
    level::LevelConfig,
    render::MOVE_SPEED,
    turn::{Turn, TurnPhase, TurnState},
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

const MIN_SPAWN_DISTANCE: i32 = 4;

// --- Shared spawn helper ---

fn spawn_enemy(
    commands: &mut Commands,
    grid: &mut HexGrid<TileData>,
    hex: Hex,
    kind: EnemyKind,
    stats: Stats,
    health: Health,
    attack_pattern: Option<AttackPattern>,
) -> Entity {
    let mut ec = commands.spawn((
        Enemy,
        GameEntity,
        kind,
        health,
        HexPosition(hex),
        Stats {
            move_range: stats.move_range,
            attack_range: stats.attack_range,
        },
    ));

    if let Some(pattern) = attack_pattern {
        ec.insert(pattern);
    }

    let entity = ec.id();

    if let Some(tile) = grid.get_mut(hex) {
        tile.occupant = Some(entity);
    }

    grid::update_ranges_with_pattern(
        grid,
        hex,
        entity,
        &stats,
        attack_pattern.as_ref(),
    );

    entity
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    player_query: Query<&HexPosition, With<Player>>,
    level_config: Res<LevelConfig>,
) {
    let Ok(player_pos) = player_query.single() else {
        return;
    };
    let player_hex = player_pos.0;
    let level = level_config.current_level();

    let mut rng = rand::rng();

    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| h.distance(player_hex) >= MIN_SPAWN_DISTANCE && is_passable(&grid, h))
        .collect();
    candidates.shuffle(&mut rng);

    let mut idx = 0;

    // Grunts
    for _ in 0..level.grunts.min(candidates.len() - idx) {
        spawn_enemy(
            &mut commands,
            &mut grid,
            candidates[idx],
            EnemyKind::Melee,
            Stats { move_range: 1, attack_range: 1 },
            Health { current: 1.0, max: 1.0 },
            None,
        );
        idx += 1;
    }

    // Archers
    for _ in 0..level.archers.min(candidates.len() - idx) {
        spawn_enemy(
            &mut commands,
            &mut grid,
            candidates[idx],
            EnemyKind::Archer,
            Stats { move_range: 1, attack_range: 4 },
            Health { current: 1.0, max: 1.0 },
            Some(archer::ATTACK_PATTERN),
        );
        idx += 1;
    }

    // Warlocks
    for _ in 0..level.warlocks.min(candidates.len() - idx) {
        spawn_enemy(
            &mut commands,
            &mut grid,
            candidates[idx],
            EnemyKind::Warlock,
            Stats { move_range: 1, attack_range: 5 },
            Health { current: 1.0, max: 1.0 },
            Some(warlock::ATTACK_PATTERN),
        );
        idx += 1;
    }

    // Bombers
    for _ in 0..level.bombers.min(candidates.len() - idx) {
        spawn_enemy(
            &mut commands,
            &mut grid,
            candidates[idx],
            EnemyKind::Bomber,
            Stats { move_range: 1, attack_range: 0 },
            Health { current: 1.0, max: 1.0 },
            None,
        );
        idx += 1;
    }

    commands.insert_resource(EnemyTurnQueue::default());
}

fn render_enemy(mut commands: Commands, query: Query<(Entity, &EnemyKind), Added<Enemy>>) {
    for (entity, kind) in &query {
        let (glyph, color) = match kind {
            EnemyKind::Melee => ("@", Color::srgb(0.776, 0.259, 0.922)),
            EnemyKind::Archer => (">", Color::srgb(0.922, 0.659, 0.259)),
            EnemyKind::Warlock => ("W", Color::srgb(0.259, 0.922, 0.659)),
            EnemyKind::Bomber => ("B", Color::srgb(0.922, 0.259, 0.259)),
        };
        commands.entity(entity).insert((
            Transform::default(),
            Text2d::new(glyph),
            TextColor(color),
            ZOffset(1.0),
        ));
    }
}

// --- Shared AI helpers ---

/// Pathfind toward `target` and take up to `max_steps` steps, stopping at occupied tiles.
/// Returns (destination, path_including_current).
pub fn pathfind_and_step(
    grid: &HexGrid<TileData>,
    current: Hex,
    target: Hex,
    max_steps: i32,
) -> (Hex, Vec<Hex>) {
    let path = grid.astar(current, target, |h| {
        if h == target {
            return true;
        }
        is_passable(grid, h)
    });

    let mut destination = current;
    let mut move_path = vec![current];

    if let Some(path) = &path {
        let steps = (path.len() - 1).min(max_steps as usize);
        for i in 1..=steps {
            let candidate = path[i];
            if grid::is_occupied(grid, candidate) {
                break;
            }
            destination = candidate;
            move_path.push(candidate);
        }
    }

    (destination, move_path)
}

/// Pick a random passable neighbor matching `filter`, or None.
pub fn random_passable_neighbor(
    grid: &HexGrid<TileData>,
    current: Hex,
    filter: impl Fn(Hex) -> bool,
) -> Option<Hex> {
    let mut neighbors: Vec<Hex> = grid
        .neighbors(current)
        .into_iter()
        .filter(|&h| is_passable(grid, h) && filter(h))
        .collect();
    let mut rng = rand::rng();
    neighbors.shuffle(&mut rng);
    neighbors.first().copied()
}

fn move_enemies(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut queue: ResMut<EnemyTurnQueue>,
    mut move_order: ResMut<crate::undo::TurnMoveOrder>,
    animating: Query<(), With<MovePath>>,
    player_query: Query<&HexPosition, With<Player>>,
    bomb_query: Query<&Bomb>,
    mut enemy_query: Query<
        (
            Entity,
            &mut HexPosition,
            &Stats,
            &EnemyKind,
            Option<&AttackPattern>,
            Option<&SkipTurn>,
        ),
        (With<Enemy>, Without<Player>, Without<Dead>),
    >,
) {
    if *turn != TurnState::Active(Turn::Enemy) {
        return;
    }

    if !animating.is_empty() {
        return;
    }

    if queue.0.is_empty() {
        queue.0 = enemy_query.iter().map(|(e, _, _, _, _, _)| e).collect();
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

    let Ok((entity, mut hex_pos, stats, kind, attack_pattern, skip)) =
        enemy_query.get_mut(enemy_entity)
    else {
        return;
    };

    // Rule 4: enemies that already attacked the player skip their movement.
    if skip.is_some() {
        commands.entity(entity).remove::<SkipTurn>();
        if queue.0.is_empty() {
            *turn = TurnState::Active(Turn::Player);
        }
        return;
    }

    let current = hex_pos.0;
    let mut did_act = false;

    // Bomber has a special action: throw bomb or move
    if *kind == EnemyKind::Bomber {
        let has_bomb = bomb_query.iter().any(|b| b.owner == entity);

        let action = if has_bomb {
            let (dest, path) = grunt::compute_move(&grid, current, player_hex, stats);
            bomber::BomberAction::Move(dest, path)
        } else {
            bomber::compute_action(&grid, current, player_hex, stats)
        };

        match action {
            bomber::BomberAction::ThrowBomb { target_hex } => {
                bomber::spawn_bomb(&mut commands, entity, target_hex);
                did_act = true;
            }
            bomber::BomberAction::Move(destination, move_path) => {
                if destination != current {
                    did_act = true;
                    move_order.0.push(entity);
                    grid::move_entity(
                        &mut commands,
                        &mut grid,
                        entity,
                        &mut hex_pos,
                        &move_path,
                        stats,
                        MOVE_SPEED,
                        attack_pattern,
                    );
                }
            }
        }
    } else {
        let (destination, move_path) = match kind {
            EnemyKind::Melee => grunt::compute_move(&grid, current, player_hex, stats),
            EnemyKind::Archer => archer::compute_move(&grid, current, player_hex, stats),
            EnemyKind::Warlock => warlock::compute_move(&grid, current, player_hex, stats),
            EnemyKind::Bomber => unreachable!(),
        };

        if destination != current {
            did_act = true;
            move_order.0.push(entity);
            grid::move_entity(
                &mut commands,
                &mut grid,
                entity,
                &mut hex_pos,
                &move_path,
                stats,
                MOVE_SPEED,
                attack_pattern,
            );
        }
    }

    if queue.0.is_empty() {
        if did_act {
            *turn = TurnState::Animating {
                next: TurnPhase::Turn(Turn::Player),
            };
        } else {
            *turn = TurnState::Active(Turn::Player);
        }
    }
}
