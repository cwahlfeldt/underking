use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    GameSettings,
    components::HexPosition,
    hex::{HEX_SIZE, Hex, HexGrid, TileData},
};

const GAP: f32 = 1.5;
const TILE_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const WALL_COLOR: Color = Color::srgba(0.2, 0.2, 0.2, 0.2);
const HIGHLIGHT_COLOR: Color = Color::srgb(0.6, 0.4, 0.15);
const GRID_RADIUS: i32 = 4;
const UNTRAVERSABLE_COUNT: usize = 10;

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tiles)
            .add_systems(Update, render_tiles);
    }
}

#[derive(Component)]
pub struct Tile;

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
    let tile_matl = materials.add(ColorMaterial::from_color(TILE_COLOR));
    let wall_matl = materials.add(ColorMaterial::from_color(WALL_COLOR));
    let hover_matl = materials.add(ColorMaterial::from_color(HIGHLIGHT_COLOR));

    for (entity, hex_pos) in &query {
        let traversable = grid.get(hex_pos.0).map(|t| t.traversable).unwrap_or(true);
        let occupant: bool = grid
            .get(hex_pos.0)
            .map(|t| t.occupant.is_some())
            .unwrap_or(false);

        let matl = if traversable {
            tile_matl.clone()
        } else {
            wall_matl.clone()
        };
        let rest_matl = matl.clone();

        let mut ec = commands.entity(entity);
        ec.insert((
            Mesh2d(hex_mesh.clone()),
            MeshMaterial2d(matl),
            Transform::default().with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_6)),
        ));

        if traversable || occupant {
            ec.insert(Pickable::default())
                .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
                .observe(update_material_on::<Pointer<Out>>(rest_matl))
                .observe(on_tile_click);
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

fn update_material_on<E: EntityEvent>(
    new_material: Handle<ColorMaterial>,
) -> impl Fn(On<E>, Query<&mut MeshMaterial2d<ColorMaterial>>) {
    move |ev, mut query| {
        if let Ok(mut material) = query.get_mut(ev.event_target()) {
            material.0 = new_material.clone();
        }
    }
}
