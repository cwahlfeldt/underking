mod components;
mod enemy;
mod hex;
mod player;
mod tile;

use crate::hex::Hex;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    selected_hex: Hex,
}

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: Hex::ORIGIN,
        })
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(tile::TilePlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(enemy::EnemyPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
