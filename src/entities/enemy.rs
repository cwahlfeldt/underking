use crate::components::{Health, HexPosition};
use crate::hex::{Hex, HexGrid, TileData};
use crate::render::hex_to_rgb;
use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemy.after(super::tile::spawn_tiles))
            .add_systems(Update, render_enemy);
    }
}

#[derive(Component)]
pub struct Enemy;

const ATTACK_RANGE: i32 = 1;

fn spawn_enemy(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    let start_coord = Hex { q: 0, r: 1, s: -1 };

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

    // Set attack range on surrounding tiles
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
