use bevy::prelude::*;

use crate::{
    GameSettings,
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

fn spawn_player(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    let start_coord = Hex { q: 0, r: -4, s: 4 };

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
    game_settings: Res<GameSettings>,
    mut query: Query<(Entity, &mut HexPosition, Has<MovePath>), With<Player>>,
) {
    let Ok((entity, mut hex_pos, is_moving)) = query.single_mut() else {
        return;
    };

    // Don't start a new move while animating
    if is_moving {
        return;
    }

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

        // Update hex position and grid state immediately to the destination
        hex_pos.0 = destination;

        if let Some(tile) = grid.get_mut(old_pos) {
            tile.occupant = None;
        }
        if let Some(tile) = grid.get_mut(destination) {
            tile.occupant = Some(entity);
        }

        clear_ranges(&mut grid, entity);
        update_ranges(&mut grid, destination, entity);

        // Build pixel waypoints for the animation
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
    }
}

fn update_ranges(grid: &mut HexGrid<TileData>, pos: Hex, entity: Entity) {
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

fn clear_ranges(grid: &mut HexGrid<TileData>, entity: Entity) {
    let positions: Vec<Hex> = grid.positions();
    for pos in positions {
        if let Some(tile) = grid.get_mut(pos) {
            tile.move_ranges.retain(|&e| e != entity);
            tile.attack_ranges.retain(|&e| e != entity);
        }
    }
}
