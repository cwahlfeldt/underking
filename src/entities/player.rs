use bevy::prelude::*;

use crate::{
    GameSettings,
    components::{Health, HexPosition},
    hex::{Hex, HexGrid, TileData},
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

const MOVE_RANGE: i32 = 2;
const ATTACK_RANGE: i32 = 1;

fn spawn_player(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    let start_coord = Hex { q: 0, r: -1, s: 1 };

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
    mut grid: ResMut<HexGrid<TileData>>,
    game_settings: Res<GameSettings>,
    mut query: Query<(Entity, &mut HexPosition), With<Player>>,
) {
    let Ok((entity, mut hex_pos)) = query.single_mut() else {
        return;
    };

    let target = game_settings.selected_hex;
    if hex_pos.0 == target {
        return;
    }

    let in_move_range = grid
        .get(target)
        .map(|t| t.move_ranges.contains(&entity))
        .unwrap_or(false);

    if !in_move_range {
        return;
    }

    let passable = |h: Hex| grid.get(h).map(|t| t.occupant.is_none()).unwrap_or(false);

    let path = grid.astar(hex_pos.0, target, passable);
    if let Some(path) = path {
        if let Some(&next) = path.get(1) {
            let old_pos = hex_pos.0;
            hex_pos.0 = next;

            // Update occupancy
            if let Some(tile) = grid.get_mut(old_pos) {
                tile.occupant = None;
            }
            if let Some(tile) = grid.get_mut(next) {
                tile.occupant = Some(entity);
            }

            // Recalculate ranges from new position
            clear_ranges(&mut grid, entity);
            update_ranges(&mut grid, next, entity);
        }
    }
}

fn update_ranges(grid: &mut HexGrid<TileData>, pos: Hex, entity: Entity) {
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

fn clear_ranges(grid: &mut HexGrid<TileData>, entity: Entity) {
    let positions: Vec<Hex> = grid.positions();
    for pos in positions {
        if let Some(tile) = grid.get_mut(pos) {
            tile.move_ranges.retain(|&e| e != entity);
            tile.attack_ranges.retain(|&e| e != entity);
        }
    }
}
