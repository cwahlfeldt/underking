use bevy::prelude::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player);
    }
}

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Health {
    current: f32,
    max: f32,
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        Player,
        Health {
            current: 3.0,
            max: 3.0,
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Text2d::new("@"),
        TextColor(Color::WHITE),
    ));
}
