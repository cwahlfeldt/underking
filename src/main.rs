mod components;
mod entities;
mod hex;
mod render;

use crate::components::MovePath;
use crate::hex::Hex;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub selected_hex: Option<Hex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turn {
    Player,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub enum TurnState {
    /// Waiting for this turn's entity to act.
    Active(Turn),
    /// An entity is animating; once done, switch to `next`.
    Animating { next: Turn },
}

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: None,
        })
        .insert_resource(TurnState::Active(Turn::Player))
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_systems(Update, check_animation_done)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// When all MovePath animations finish, advance to the next turn.
fn check_animation_done(
    mut turn: ResMut<TurnState>,
    animating: Query<(), With<MovePath>>,
) {
    let TurnState::Animating { next } = *turn else {
        return;
    };

    if animating.is_empty() {
        *turn = TurnState::Active(next);
    }
}
