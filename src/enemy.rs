use crate::components::health::Health;
use crate::consts::HEX_SIZE;
use crate::hex::Hex;
use crate::util::hex_to_rgb;
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
    let start_coord = Hex { q: 0, r: 1, s: -1 };
    let (x, y) = start_coord.to_pixel(HEX_SIZE);

    // Enemy entity
    commands.spawn((
        Enemy,
        Health {
            current: 3.0,
            max: 3.0,
        },
        Transform::from_xyz(x, y, 0.0),
        Text2d::new("@"),
        TextColor(Color::srgb(r, g, b)),
    ));
}
