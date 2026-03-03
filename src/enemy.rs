use crate::components::health::Health;
use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_enemy);
    }
}

#[derive(Component)]
pub struct Enemy;

fn spawn_enemy(mut commands: Commands) {
    let (r, g, b) = hex_to_rgb("#c642eb");

    commands.spawn((
        Enemy,
        Health {
            current: 3.0,
            max: 3.0,
        },
        Transform::from_xyz(0.0, 50.0, 0.0),
        Text2d::new("@"),
        TextColor(Color::srgb(r, g, b)),
    ));
}

fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    if hex.len() != 7 || !hex.starts_with('#') {
        panic!("Invalid hex color: {hex}. Expected format: #RRGGBB");
    }

    let hex = hex.trim_start_matches("#");
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.0;

    (r, g, b)
}
