use bevy::prelude::*;

use crate::{
    GameSettings,
    consts::HEX_SIZE,
    hex::{Hex, HexGrid},
};

const GAP: f32 = 1.5;
const TILE_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const HIGHLIGHT_COLOR: Color = Color::srgb(0.6, 0.4, 0.15);

pub struct TilePlugin;

impl Plugin for TilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_tiles);
    }
}

#[derive(Component)]
pub struct Tile {
    pub hex: Hex,
}

fn spawn_tiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let grid: HexGrid<Entity> = HexGrid::new(4);

    let hex_mesh = meshes.add(RegularPolygon::new(HEX_SIZE - GAP, 6));
    let tile_matl = materials.add(ColorMaterial::from_color(TILE_COLOR));
    let hover_matl = materials.add(ColorMaterial::from_color(HIGHLIGHT_COLOR));

    for pos in grid.positions() {
        let (x, y) = pos.to_pixel(HEX_SIZE);

        commands
            .spawn((
                Tile { hex: pos },
                Mesh2d(hex_mesh.clone()),
                MeshMaterial2d(tile_matl.clone()),
                Transform::from_xyz(x, y, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_6)),
                Pickable::default(),
            ))
            .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
            .observe(update_material_on::<Pointer<Out>>(tile_matl.clone()))
            .observe(on_tile_click);
    }
}

fn on_tile_click(ev: On<Pointer<Click>>, query: Query<&Tile>, mut settings: ResMut<GameSettings>) {
    if let Ok(tile) = query.get(ev.event_target()) {
        info!("Clicked hex: {:?}", tile.hex);
        settings.selected_hex = tile.hex;
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
