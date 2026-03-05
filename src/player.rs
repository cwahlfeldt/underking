use bevy::prelude::*;

use crate::{GameSettings, components::health::Health, consts::HEX_SIZE, hex::Hex};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, move_player);
    }
}

#[derive(Component)]
pub struct Player;

fn spawn_player(mut commands: Commands) {
    let start_coord = Hex { q: 0, r: -1, s: 1 };
    let (x, y) = start_coord.to_pixel(HEX_SIZE);

    commands.spawn((
        Player,
        Health {
            current: 3.0,
            max: 3.0,
        },
        Transform::from_xyz(x, y, 0.0),
        Text2d::new("@"),
        TextColor(Color::WHITE),
    ));
}

fn move_player(
    game_settings: Res<GameSettings>,
    mut transform: Single<&mut Transform, With<Player>>,
) {
    let (x, y) = game_settings.selected_hex.to_pixel(HEX_SIZE);
    transform.translation = Vec3::new(x, y, 0.0);
}
