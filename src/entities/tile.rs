use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    GameSettings,
    components::HexPosition,
    entities::enemy::Enemy,
    grid::TileData,
    hex::{HEX_SIZE, Hex, HexGrid},
};

const GAP: f32 = 1.5;
const TILE_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const WALL_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.2);
const HIGHLIGHT_COLOR: Color = Color::srgba(0.2, 0.2, 0.8, 0.8);
const ATTACK_RANGE_COLOR: Color = Color::srgb(0.6, 0.15, 0.15);
pub const GRID_RADIUS: i32 = 4;
const UNTRAVERSABLE_COUNT: usize = 10;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tiles)
            .add_systems(Update, (render_tiles, show_enemy_attack_range));
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
