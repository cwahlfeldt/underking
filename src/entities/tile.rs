use bevy::prelude::*;
use rand::seq::SliceRandom;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};

use crate::{
    components::{Bomb, GameEntity, HexPosition},
    entities::enemy::Enemy,
    grid::TileData,
    hex::{HEX_SIZE, ISO_Y_SCALE, Hex, HexGrid},
    level::LevelConfig,
    turn::GameSettings,
};

const GAP: f32 = 1.5;
const EXTRUDE_HEIGHT: f32 = 18.0;
const OUTLINE_THICKNESS: f32 = 3.0;
const TILE_COLOR: Color = Color::srgb(0.25, 0.25, 0.25);
const TILE_SIDE_COLOR: Color = Color::srgb(0.13, 0.13, 0.13);
const WALL_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.2);
const WALL_SIDE_COLOR: Color = Color::srgba(0.1, 0.1, 0.1, 0.2);
const OUTLINE_DEFAULT_COLOR: Color = Color::srgba(0.4, 0.4, 0.4, 0.6);
const HIGHLIGHT_COLOR: Color = Color::srgba(0.3, 0.3, 1.0, 1.0);
const ATTACK_RANGE_COLOR: Color = Color::srgb(0.8, 0.15, 0.15);

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tiles)
            .add_systems(Update, (render_tiles, show_enemy_attack_range));
    }
}

#[derive(Component)]
pub struct Tile;

/// Stores each tile's default (non-hovered) outline material for reliable restoration.
#[derive(Component)]
struct RestOutlineMaterial(Handle<ColorMaterial>);

/// Points from the parent tile entity to its outline child entity.
#[derive(Component)]
struct OutlineChild(Entity);

pub fn spawn_tiles(mut commands: Commands, level_config: Res<LevelConfig>) {
    let level = level_config.current_level();
    let grid_radius = level.grid_radius;
    let wall_count = level.walls;

    let mut grid: HexGrid<TileData> = HexGrid::new(grid_radius);

    // Pick random non-origin tiles to be untraversable
    let mut rng = rand::rng();
    let positions = grid.positions();
    let mut candidates: Vec<_> = positions
        .iter()
        .filter(|&&h| h != Hex::ORIGIN)
        .copied()
        .collect();
    candidates.shuffle(&mut rng);
    let walls: Vec<Hex> = candidates.into_iter().take(wall_count).collect();

    for pos in &positions {
        let traversable = !walls.contains(pos);
        let entity = commands.spawn((Tile, GameEntity, HexPosition(*pos))).id();

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
    let hex_mesh = meshes.add(create_iso_hex_mesh(HEX_SIZE - GAP));
    let side_mesh = meshes.add(create_hex_side_mesh(HEX_SIZE - GAP, EXTRUDE_HEIGHT));
    let outline_mesh = meshes.add(create_hex_outline_mesh(HEX_SIZE - GAP, OUTLINE_THICKNESS));
    let hover_matl = materials.add(ColorMaterial::from_color(HIGHLIGHT_COLOR));

    for (entity, hex_pos) in &query {
        let traversable = grid.get(hex_pos.0).map(|t| t.traversable).unwrap_or(true);
        let occupant: bool = grid
            .get(hex_pos.0)
            .map(|t| t.occupant.is_some())
            .unwrap_or(false);

        let (top_color, side_color) = if traversable {
            (TILE_COLOR, TILE_SIDE_COLOR)
        } else {
            (WALL_COLOR, WALL_SIDE_COLOR)
        };

        let matl = materials.add(ColorMaterial::from_color(top_color));
        let side_matl = materials.add(ColorMaterial::from_color(side_color));
        let outline_matl = materials.add(ColorMaterial::from_color(OUTLINE_DEFAULT_COLOR));
        let rest_outline_matl = outline_matl.clone();

        // Spawn outline child entity
        let outline_entity = commands
            .spawn((
                Mesh2d(outline_mesh.clone()),
                MeshMaterial2d(outline_matl),
                Transform::from_xyz(0.0, 0.0, 0.1),
                Pickable::IGNORE,
            ))
            .id();

        let mut ec = commands.entity(entity);
        ec.insert((
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(matl),
            OutlineChild(outline_entity),
            RestOutlineMaterial(rest_outline_matl.clone()),
            Transform::default(),
        ));
        ec.add_child(outline_entity);

        // Spawn the extruded side as a child so it moves with the tile.
        // z = -0.1 so it renders behind the top face.
        ec.with_child((
            Mesh2d(side_mesh.clone()),
            MeshMaterial2d(side_matl),
            Transform::from_xyz(0.0, 0.0, -0.1),
            Pickable::IGNORE,
        ));

        if traversable || occupant {
            ec.insert(Pickable::default())
                .observe(update_outline_on::<Pointer<Over>>(hover_matl.clone()))
                .observe(update_outline_on::<Pointer<Out>>(rest_outline_matl))
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
    bomb_query: Query<(Entity, &HexPosition), With<Bomb>>,
    mut settings: ResMut<GameSettings>,
) {
    if let Ok(pos) = query.get(ev.event_target()) {
        // Check for enemy occupant
        if let Some(tile) = grid.get(pos.0) {
            if let Some(occupant) = tile.occupant {
                if enemy_query.get(occupant).is_ok() {
                    settings.hovered_enemy = Some(occupant);
                }
            }
        }
        // Check for bomb on this hex
        for (bomb_entity, bomb_pos) in &bomb_query {
            if bomb_pos.0 == pos.0 {
                settings.hovered_bomb = Some(bomb_entity);
            }
        }
    }
}

fn on_tile_out(
    ev: On<Pointer<Out>>,
    query: Query<&HexPosition>,
    grid: Res<HexGrid<TileData>>,
    enemy_query: Query<(), With<Enemy>>,
    bomb_query: Query<(Entity, &HexPosition), With<Bomb>>,
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
        for (bomb_entity, bomb_pos) in &bomb_query {
            if bomb_pos.0 == pos.0 {
                if settings.hovered_bomb == Some(bomb_entity) {
                    settings.hovered_bomb = None;
                }
            }
        }
    }
}

const BOMB_RANGE_COLOR: Color = Color::srgb(0.9, 0.4, 0.0);

/// Highlights tile outlines in an enemy's attack range or bomb blast radius when hovered.
fn show_enemy_attack_range(
    settings: Res<GameSettings>,
    grid: Res<HexGrid<TileData>>,
    bomb_query: Query<(&Bomb, &HexPosition)>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    tile_query: Query<
        (
            &HexPosition,
            &OutlineChild,
            &RestOutlineMaterial,
        ),
        With<Tile>,
    >,
    mut outline_mat_query: Query<&mut MeshMaterial2d<ColorMaterial>, Without<Tile>>,
) {
    if !settings.is_changed() {
        return;
    }

    // Enemy attack range hexes
    let mut attack_hexes = Vec::new();
    if let Some(enemy_entity) = settings.hovered_enemy {
        for pos in grid.positions() {
            if let Some(tile) = grid.get(pos) {
                if tile.attack_ranges.contains(&enemy_entity) {
                    attack_hexes.push(pos);
                }
            }
        }
    }

    // Bomb blast radius hexes
    let mut bomb_hexes = Vec::new();
    if let Some(bomb_entity) = settings.hovered_bomb {
        if let Ok((bomb, bomb_pos)) = bomb_query.get(bomb_entity) {
            bomb_hexes = bomb_pos.0.spiral(bomb.blast_radius);
        }
    }

    for (hex_pos, outline_child, rest) in &tile_query {
        let is_traversable = grid.get(hex_pos.0).map(|t| t.traversable).unwrap_or(true);
        if !is_traversable {
            continue;
        }

        if let Ok(mut mat_handle) = outline_mat_query.get_mut(outline_child.0) {
            if bomb_hexes.contains(&hex_pos.0) {
                mat_handle.0 = materials.add(ColorMaterial::from_color(BOMB_RANGE_COLOR));
            } else if attack_hexes.contains(&hex_pos.0) {
                mat_handle.0 = materials.add(ColorMaterial::from_color(ATTACK_RANGE_COLOR));
            } else {
                mat_handle.0 = rest.0.clone();
            }
        }
    }
}

fn update_outline_on<E: EntityEvent>(
    new_material: Handle<ColorMaterial>,
) -> impl Fn(On<E>, Query<&OutlineChild>, Query<&mut MeshMaterial2d<ColorMaterial>, Without<Tile>>)
{
    move |ev, outline_query, mut mat_query| {
        if let Ok(outline_child) = outline_query.get(ev.event_target()) {
            if let Ok(mut material) = mat_query.get_mut(outline_child.0) {
                material.0 = new_material.clone();
            }
        }
    }
}

/// Build the extruded side faces for a hex tile (visible bottom 3 edges).
/// Creates quads that drop down from the bottom edges of the hex top face.
fn create_hex_side_mesh(size: f32, height: f32) -> Mesh {
    // Flat-topped hex vertices (same as top face, indices 1..=6 in the top mesh)
    let hex_verts: Vec<[f32; 2]> = (0..6)
        .map(|i| {
            let angle_rad = (60.0 * i as f32).to_radians();
            [size * angle_rad.cos(), size * angle_rad.sin() * ISO_Y_SCALE]
        })
        .collect();

    // Bottom 3 edges of the hex (visible from above in iso view).
    // For a flat-top hex: edges connecting vertices 2→3, 3→4, 4→5
    // (these are the lower-left, bottom, and lower-right edges)
    let visible_edges: &[(usize, usize)] = &[(2, 3), (3, 4), (4, 5)];

    let mut positions = Vec::new();
    let mut indices = Vec::new();

    for &(a, b) in visible_edges {
        let base = positions.len() as u32;
        let [ax, ay] = hex_verts[a];
        let [bx, by] = hex_verts[b];

        // Quad: top-left, top-right, bottom-right, bottom-left
        positions.push([ax, ay, 0.0]);
        positions.push([bx, by, 0.0]);
        positions.push([bx, by - height, 0.0]);
        positions.push([ax, ay - height, 0.0]);

        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let normals: Vec<[f32; 3]> = (0..positions.len()).map(|_| [0.0, 0.0, 1.0]).collect();
    let uvs: Vec<[f32; 2]> = positions.iter().map(|_| [0.0, 0.0]).collect();

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Build a hex outline (ring) mesh — only the border, not filled.
fn create_hex_outline_mesh(size: f32, thickness: f32) -> Mesh {
    let outer = size;
    let inner = size - thickness;

    let mut positions = Vec::with_capacity(12);
    let mut indices = Vec::new();

    // Generate outer and inner vertices interleaved
    for i in 0..6 {
        let angle_rad = (60.0 * i as f32).to_radians();
        let cos = angle_rad.cos();
        let sin = angle_rad.sin() * ISO_Y_SCALE;
        positions.push([outer * cos, outer * sin, 0.0]);
        positions.push([inner * cos, inner * sin, 0.0]);
    }

    // Create quads for each edge segment
    for i in 0..6u32 {
        let next = (i + 1) % 6;
        let o1 = i * 2; // outer vertex i
        let i1 = i * 2 + 1; // inner vertex i
        let o2 = next * 2; // outer vertex i+1
        let i2 = next * 2 + 1; // inner vertex i+1

        // Two triangles per quad
        indices.extend_from_slice(&[o1, o2, i2, o1, i2, i1]);
    }

    let normals: Vec<[f32; 3]> = (0..positions.len()).map(|_| [0.0, 0.0, 1.0]).collect();
    let uvs: Vec<[f32; 2]> = positions.iter().map(|_| [0.0, 0.0]).collect();

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}

/// Build a flat-topped hex mesh with isometric y-squash baked into vertices.
fn create_iso_hex_mesh(size: f32) -> Mesh {
    let mut positions = Vec::with_capacity(7);
    positions.push([0.0_f32, 0.0, 0.0]); // center

    for i in 0..6 {
        let angle_rad = (60.0 * i as f32).to_radians();
        let x = size * angle_rad.cos();
        let y = size * angle_rad.sin() * ISO_Y_SCALE;
        positions.push([x, y, 0.0]);
    }

    // Fan triangles from center vertex
    let indices: Vec<u32> = (0..6u32)
        .flat_map(|i| [0, i + 1, if i + 2 > 6 { 1 } else { i + 2 }])
        .collect();

    let normals: Vec<[f32; 3]> = (0..7).map(|_| [0.0, 0.0, 1.0]).collect();
    let uvs: Vec<[f32; 2]> = positions
        .iter()
        .map(|v| [(v[0] / size + 1.0) * 0.5, (v[1] / size + 1.0) * 0.5])
        .collect();

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}
