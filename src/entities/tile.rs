use std::collections::HashSet;

use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    GameSettings, Turn, TurnState,
    components::HexPosition,
    entities::{enemy::Enemy, player::Player},
    grid::TileData,
    hex::{HEX_SIZE, Hex, HexGrid},
};

const GAP: f32 = 1.5;
const TILE_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const WALL_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.2);
const HIGHLIGHT_COLOR: Color = Color::srgba(0.2, 0.2, 0.8, 0.8);
const ATTACK_RANGE_COLOR: Color = Color::srgb(0.6, 0.15, 0.15);
/// Tiles the player can move to (no adjacent enemy).
const PLAYER_MOVE_COLOR: Color = Color::srgba(0.15, 0.35, 0.6, 0.55);
/// Tiles the player can move to that will trigger an attack on an adjacent enemy.
const PLAYER_ATTACK_COLOR: Color = Color::srgba(0.8, 0.4, 0.1, 0.75);
pub const GRID_RADIUS: i32 = 4;
const UNTRAVERSABLE_COUNT: usize = 10;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tiles).add_systems(
            Update,
            (render_tiles, show_player_ranges, show_enemy_attack_range).chain(),
        );
    }
}

#[derive(Component)]
pub struct Tile;

/// Stores each tile's default (non-hovered) material handle for reliable restoration.
#[derive(Component)]
struct RestMaterial(Handle<ColorMaterial>);

pub fn spawn_tiles(mut commands: Commands) {
    let mut grid: HexGrid<TileData> = HexGrid::new(GRID_RADIUS);

    // Pick random non-origin tiles to be untraversable
    let mut rng = rand::rng();
    let positions = grid.positions();
    let mut candidates: Vec<_> = positions
        .iter()
        .filter(|&&h| h != Hex::ORIGIN)
        .copied()
        .collect();
    candidates.shuffle(&mut rng);
    let walls: Vec<Hex> = candidates.into_iter().take(UNTRAVERSABLE_COUNT).collect();

    for pos in &positions {
        let traversable = !walls.contains(pos);
        let entity = commands.spawn((Tile, HexPosition(*pos))).id();

        grid.insert(
            *pos,
            TileData {
                tile_entity: Some(entity),
                traversable,
                ..default()
            },
        );
    }

    commands.insert_resource(grid);
}

fn render_tiles(
    mut commands: Commands,
    query: Query<(Entity, &HexPosition), Added<Tile>>,
    grid: Res<HexGrid<TileData>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let hex_mesh = meshes.add(RegularPolygon::new(HEX_SIZE - GAP, 6));
    let hover_matl = materials.add(ColorMaterial::from_color(HIGHLIGHT_COLOR));

    for (entity, hex_pos) in &query {
        let traversable = grid.get(hex_pos.0).map(|t| t.traversable).unwrap_or(true);
        let occupant: bool = grid
            .get(hex_pos.0)
            .map(|t| t.occupant.is_some())
            .unwrap_or(false);

        // Each tile gets its own material so we can tint individually
        let matl = if traversable {
            materials.add(ColorMaterial::from_color(TILE_COLOR))
        } else {
            materials.add(ColorMaterial::from_color(WALL_COLOR))
        };
        let rest_matl = matl.clone();

        let mut ec = commands.entity(entity);
        ec.insert((
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(matl.clone()),
            RestMaterial(matl),
            Transform::default().with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_6)),
        ));

        if traversable || occupant {
            ec.insert(Pickable::default())
                .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
                .observe(update_material_on::<Pointer<Out>>(rest_matl))
                .observe(on_tile_click)
                .observe(on_tile_over)
                .observe(on_tile_out);
        } else {
            ec.insert(Pickable::IGNORE);
        }
    }
}

fn on_tile_click(
    ev: On<Pointer<Click>>,
    query: Query<&HexPosition>,
    mut settings: ResMut<GameSettings>,
) {
    if let Ok(pos) = query.get(ev.event_target()) {
        info!("Clicked hex: {:?}", pos.0);
        settings.selected_hex = Some(pos.0);
    }
}

fn on_tile_over(
    ev: On<Pointer<Over>>,
    query: Query<&HexPosition>,
    grid: Res<HexGrid<TileData>>,
    enemy_query: Query<(), With<Enemy>>,
    mut settings: ResMut<GameSettings>,
) {
    if let Ok(pos) = query.get(ev.event_target()) {
        if let Some(tile) = grid.get(pos.0) {
            if let Some(occupant) = tile.occupant {
                if enemy_query.get(occupant).is_ok() {
                    settings.hovered_enemy = Some(occupant);
                }
            }
        }
    }
}

fn on_tile_out(
    ev: On<Pointer<Out>>,
    query: Query<&HexPosition>,
    grid: Res<HexGrid<TileData>>,
    enemy_query: Query<(), With<Enemy>>,
    mut settings: ResMut<GameSettings>,
) {
    if let Ok(pos) = query.get(ev.event_target()) {
        if let Some(tile) = grid.get(pos.0) {
            if let Some(occupant) = tile.occupant {
                if enemy_query.get(occupant).is_ok() {
                    if settings.hovered_enemy == Some(occupant) {
                        settings.hovered_enemy = None;
                    }
                }
            }
        }
    }
}

/// Highlights the player's reachable tiles during their turn.
/// - Orange: moving here will attack an adjacent enemy.
/// - Blue: normal move tile.
/// Also updates `RestMaterial` so hover-out restores the range color, not the
/// base tile color.
fn show_player_ranges(
    turn: Res<TurnState>,
    grid: Res<HexGrid<TileData>>,
    player_query: Query<(Entity, &HexPosition), With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut tile_query: Query<
        (
            &HexPosition,
            &mut MeshMaterial2d<ColorMaterial>,
            &mut RestMaterial,
        ),
        With<Tile>,
    >,
    mut move_matl: Local<Option<Handle<ColorMaterial>>>,
    mut attack_matl: Local<Option<Handle<ColorMaterial>>>,
    mut tile_matl: Local<Option<Handle<ColorMaterial>>>,
) {
    if !turn.is_changed() && !grid.is_changed() {
        return;
    }

    if move_matl.is_none() {
        *move_matl = Some(materials.add(ColorMaterial::from_color(PLAYER_MOVE_COLOR)));
    }
    if attack_matl.is_none() {
        *attack_matl = Some(materials.add(ColorMaterial::from_color(PLAYER_ATTACK_COLOR)));
    }
    if tile_matl.is_none() {
        *tile_matl = Some(materials.add(ColorMaterial::from_color(TILE_COLOR)));
    }
    let move_h = move_matl.as_ref().unwrap().clone();
    let attack_h = attack_matl.as_ref().unwrap().clone();
    let tile_h = tile_matl.as_ref().unwrap().clone();

    let (attack_tiles, move_tiles): (HashSet<Hex>, HashSet<Hex>) =
        if *turn == TurnState::Active(Turn::Player) {
            let Ok((player_entity, _)) = player_query.single() else {
                return;
            };
            let mut at = HashSet::new();
            let mut mt = HashSet::new();
            for pos in grid.positions() {
                let Some(tile) = grid.get(pos) else {
                    continue;
                };
                if !tile.traversable || tile.occupant.is_some() {
                    continue;
                }
                if !tile.move_ranges.contains(&player_entity) {
                    continue;
                }
                let adj_enemy = grid.neighbors(pos).into_iter().any(|n| {
                    grid.get(n)
                        .and_then(|t| t.occupant)
                        .map(|occ| enemy_query.contains(occ))
                        .unwrap_or(false)
                });
                if adj_enemy {
                    at.insert(pos);
                } else {
                    mt.insert(pos);
                }
            }
            (at, mt)
        } else {
            (HashSet::new(), HashSet::new())
        };

    for (hex_pos, mut mat, mut rest) in &mut tile_query {
        let hex = hex_pos.0;
        if !grid.get(hex).map(|t| t.traversable).unwrap_or(true) {
            continue;
        }
        if attack_tiles.contains(&hex) {
            rest.0 = attack_h.clone();
            mat.0 = attack_h.clone();
        } else if move_tiles.contains(&hex) {
            rest.0 = move_h.clone();
            mat.0 = move_h.clone();
        } else {
            rest.0 = tile_h.clone();
            mat.0 = tile_h.clone();
        }
    }
}

/// Highlights tiles in an enemy's attack range when hovered.
fn show_enemy_attack_range(
    settings: Res<GameSettings>,
    grid: Res<HexGrid<TileData>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut tile_query: Query<
        (
            &HexPosition,
            &mut MeshMaterial2d<ColorMaterial>,
            &RestMaterial,
        ),
        With<Tile>,
    >,
) {
    if !settings.is_changed() {
        return;
    }

    // Collect which hexes should be highlighted
    let mut highlighted_hexes = Vec::new();
    if let Some(enemy_entity) = settings.hovered_enemy {
        for pos in grid.positions() {
            if let Some(tile) = grid.get(pos) {
                if tile.attack_ranges.contains(&enemy_entity) {
                    highlighted_hexes.push(pos);
                }
            }
        }
    }

    for (hex_pos, mut mat_handle, rest) in &mut tile_query {
        let is_in_range = highlighted_hexes.contains(&hex_pos.0);
        let is_traversable = grid.get(hex_pos.0).map(|t| t.traversable).unwrap_or(true);

        if !is_traversable {
            continue;
        }

        if is_in_range {
            let attack_matl = materials.add(ColorMaterial::from_color(ATTACK_RANGE_COLOR));
            mat_handle.0 = attack_matl;
        } else {
            // Restore to the tile's original material
            mat_handle.0 = rest.0.clone();
        }
    }
}

fn update_material_on<E: EntityEvent>(
    new_material: Handle<ColorMaterial>,
) -> impl Fn(On<E>, Query<&mut MeshMaterial2d<ColorMaterial>>) {
    move |ev, mut query| {
        if let Ok(mut material) = query.get_mut(ev.event_target()) {
            material.0 = new_material.clone();
        }
    }
}
