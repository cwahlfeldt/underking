mod components;
mod entities;
mod hex;
mod render;

use crate::hex::Hex;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub selected_hex: Hex,
}

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: Hex::ORIGIN,
        })
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
