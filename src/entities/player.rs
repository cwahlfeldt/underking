use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{Health, HexPosition, Stats},
    grid::{TileData, is_passable, move_entity, update_ranges},
    hex::{Hex, HexGrid},
    render::MOVE_SPEED,
    turn::{CombatPhase, GameSettings, Turn, TurnPhase, TurnState},
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player.after(super::tile::spawn_tiles))
            .add_systems(Update, (render_player, handle_player_move).chain());
    }
}

#[derive(Component)]
pub struct Player;

pub fn spawn_player(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    let mut rng = rand::rng();
    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| is_passable(&grid, h))
        .collect();
    candidates.shuffle(&mut rng);
    let start_coord = candidates[0];

    let stats = Stats {
        move_range: 1,
        attack_range: 1,
    };

    let entity = commands
        .spawn((
            Player,
            Health {
                current: 3.0,
                max: 3.0,
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

fn render_player(mut commands: Commands, query: Query<Entity, Added<Player>>) {
    for entity in &query {
        commands.entity(entity).insert((
            Transform::default(),
            Text2d::new("@"),
            TextColor(Color::WHITE),
        ));
    }
}

fn handle_player_move(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut game_settings: ResMut<GameSettings>,
    mut history: ResMut<crate::undo::UndoHistory>,
    mut move_order: ResMut<crate::undo::TurnMoveOrder>,
    mut query: Query<(Entity, &mut HexPosition, &Stats), With<Player>>,
) {
    if *turn != TurnState::Active(Turn::Player) {
        return;
    }

    let Ok((entity, mut hex_pos, stats)) = query.single_mut() else {
        return;
    };

    let Some(target) = game_settings.selected_hex else {
        return;
    };
    if hex_pos.0 == target {
        return;
    }

    let can_move = grid
        .get(target)
        .map(|t| {
            t.traversable
                && t.occupant.is_none()
                && (stats.move_range == 0 || t.move_ranges.contains(&entity))
        })
        .unwrap_or(false);

    if !can_move {
        return;
    }

    let path = grid.astar(hex_pos.0, target, |h| is_passable(&grid, h));

    if let Some(path) = path {
        let snapshot = crate::undo::capture_snapshot(&grid, &turn, &move_order);
        crate::undo::push_undo(&mut history, snapshot);

        move_order.0.clear();
        move_order.0.push(entity);

        game_settings.selected_hex = None;
        game_settings.player_prev_hex = Some(hex_pos.0);
        move_entity(
            &mut commands,
            &mut grid,
            &mut turn,
            entity,
            &mut hex_pos,
            &path,
            stats,
            MOVE_SPEED,
            TurnPhase::Combat(CombatPhase::AfterPlayerMove),
        );
    }
}
