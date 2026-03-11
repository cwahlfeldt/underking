use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    GameSettings, Turn, TurnState,
    components::{Health, HexPosition, MovePath},
    hex::{HEX_SIZE, Hex, HexGrid, TileData},
    render::MOVE_SPEED,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player.after(super::tile::spawn_tiles))
            .add_systems(Update, (render_player, move_player).chain());
    }
}

#[derive(Component)]
pub struct Player;

const MOVE_RANGE: i32 = 0; // 0 is any tile
const ATTACK_RANGE: i32 = 1;

pub fn spawn_player(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    // Pick a random traversable, unoccupied tile for the player
    let mut rng = rand::rng();
    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| {
            grid.get(h)
                .map(|t| t.traversable && t.occupant.is_none())
                .unwrap_or(false)
        })
        .collect();
    candidates.shuffle(&mut rng);
    let start_coord = candidates[0];

    let entity = commands
        .spawn((
            Player,
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

    update_ranges(&mut grid, start_coord, entity);
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

fn move_player(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut game_settings: ResMut<GameSettings>,
    mut query: Query<(Entity, &mut HexPosition), With<Player>>,
) {
    // Only act on player's turn
    if *turn != TurnState::Active(Turn::Player) {
        return;
    }

    let Ok((entity, mut hex_pos)) = query.single_mut() else {
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
                && (MOVE_RANGE == 0 || t.move_ranges.contains(&entity))
        })
        .unwrap_or(false);

    if !can_move {
        return;
    }

    let path = grid.astar(hex_pos.0, target, |h| {
        grid.get(h)
            .map(|t| t.traversable && t.occupant.is_none())
            .unwrap_or(false)
    });

    if let Some(path) = path {
        if path.len() < 2 {
            return;
        }

        let destination = *path.last().unwrap();
        let old_pos = hex_pos.0;

        hex_pos.0 = destination;

        if let Some(tile) = grid.get_mut(old_pos) {
            tile.occupant = None;
        }
        if let Some(tile) = grid.get_mut(destination) {
            tile.occupant = Some(entity);
        }

        clear_ranges(&mut grid, entity);
        update_ranges(&mut grid, destination, entity);

        let waypoints: Vec<Vec2> = path
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

        // Clear selection and transition to animating → enemy turn next
        game_settings.selected_hex = None;
        *turn = TurnState::Animating {
            next: Turn::Enemy,
        };
    }
}

pub fn update_ranges(grid: &mut HexGrid<TileData>, pos: Hex, entity: Entity) {
    if MOVE_RANGE > 0 {
        let move_hexes: Vec<Hex> = pos
            .spiral(MOVE_RANGE)
            .into_iter()
            .filter(|&h| h != pos && grid.contains(h))
            .collect();

        for hex in move_hexes {
            if let Some(tile) = grid.get_mut(hex) {
                tile.move_ranges.push(entity);
            }
        }
    }

    let attack_hexes: Vec<Hex> = pos
        .spiral(ATTACK_RANGE)
        .into_iter()
        .filter(|&h| h != pos && grid.contains(h))
        .collect();

    for hex in attack_hexes {
        if let Some(tile) = grid.get_mut(hex) {
            tile.attack_ranges.push(entity);
        }
    }
}

pub fn clear_ranges(grid: &mut HexGrid<TileData>, entity: Entity) {
    let positions: Vec<Hex> = grid.positions();
    for pos in positions {
        if let Some(tile) = grid.get_mut(pos) {
            tile.move_ranges.retain(|&e| e != entity);
            tile.attack_ranges.retain(|&e| e != entity);
        }
    }
}
