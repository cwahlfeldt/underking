pub mod archer;
pub mod bomber;
pub mod grunt;
pub mod warlock;

use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{
        AttackPattern, Bomb, Dead, EnemyKind, HexPosition, MovePath, SkipTurn, Stats, ZOffset,
    },
    entities::player::Player,
    grid::{TileData, clear_ranges, is_passable, update_ranges_with_pattern},
    hex::{HEX_SIZE, Hex, HexGrid},
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

    let grunt_count = level.grunts.min(candidates.len());
    grunt::spawn_grunts(&mut commands, &mut grid, &candidates[..grunt_count]);

    let after_grunts = &candidates[grunt_count..];
    let archer_count = level.archers.min(after_grunts.len());
    archer::spawn_archers(&mut commands, &mut grid, &after_grunts[..archer_count]);

    let after_archers = &after_grunts[archer_count..];
    let warlock_count = level.warlocks.min(after_archers.len());
    warlock::spawn_warlocks(&mut commands, &mut grid, &after_archers[..warlock_count]);

    let after_warlocks = &after_archers[warlock_count..];
    let bomber_count = level.bombers.min(after_warlocks.len());
    bomber::spawn_bombers(&mut commands, &mut grid, &after_warlocks[..bomber_count]);

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
        // Check if this bomber already has a bomb out
        let has_bomb = bomb_query.iter().any(|b| b.owner == entity);

        let action = if has_bomb {
            // Can't throw — just move instead
            let (dest, path) = grunt::compute_move(&grid, current, player_hex, stats);
            bomber::BomberAction::Move(dest, path)
        } else {
            bomber::compute_action(&grid, current, player_hex, stats)
        };

        match action {
            bomber::BomberAction::ThrowBomb {
                bomber_hex: _,
                target_hex,
            } => {
                bomber::spawn_bomb(&mut commands, entity, target_hex);
                did_act = true;
                // Bomber stays in place after throwing
            }
            bomber::BomberAction::Move(destination, move_path) => {
                if destination != current {
                    did_act = true;
                    apply_move(
                        &mut commands,
                        &mut grid,
                        &mut move_order,
                        entity,
                        &mut hex_pos,
                        destination,
                        &move_path,
                        stats,
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
            apply_move(
                &mut commands,
                &mut grid,
                &mut move_order,
                entity,
                &mut hex_pos,
                destination,
                &move_path,
                stats,
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

/// Shared helper: apply a movement for an enemy entity.
fn apply_move(
    commands: &mut Commands,
    grid: &mut HexGrid<TileData>,
    move_order: &mut crate::undo::TurnMoveOrder,
    entity: Entity,
    hex_pos: &mut HexPosition,
    destination: Hex,
    move_path: &[Hex],
    stats: &Stats,
    attack_pattern: Option<&AttackPattern>,
) {
    move_order.0.push(entity);

    let old_pos = hex_pos.0;
    hex_pos.0 = destination;

    if let Some(tile) = grid.get_mut(old_pos) {
        tile.occupant = None;
    }
    if let Some(tile) = grid.get_mut(destination) {
        tile.occupant = Some(entity);
    }

    clear_ranges(grid, entity);
    update_ranges_with_pattern(grid, destination, entity, stats, attack_pattern);

    let waypoints: Vec<Vec2> = move_path
        .iter()
        .map(|h| {
            let (x, y) = h.to_iso_pixel(HEX_SIZE);
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
