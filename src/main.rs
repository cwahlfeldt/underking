mod components;
mod debug_ui;
mod entities;
mod grid;
mod hex;
mod render;
mod undo;

use crate::components::MovePath;
use crate::hex::Hex;
use bevy::dev_tools::fps_overlay::FpsOverlayPlugin;
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameSettings {
    pub selected_hex: Option<Hex>,
    pub hovered_enemy: Option<Entity>,
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
            hovered_enemy: None,
        })
        .insert_resource(TurnState::Active(Turn::Player))
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            // FpsOverlayPlugin::default(),
        ))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_plugins(debug_ui::DebugUiPlugin)
        .add_plugins(undo::UndoPlugin)
        .add_systems(Update, check_animation_done)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// When all MovePath animations finish, advance to the next turn.
fn check_animation_done(mut turn: ResMut<TurnState>, animating: Query<(), With<MovePath>>) {
    let TurnState::Animating { next } = *turn else {
        return;
    };

    if animating.is_empty() {
        *turn = TurnState::Active(next);
    }
}
