mod combat;
mod components;
mod debug_ui;
mod entities;
mod grid;
#[allow(dead_code)]
mod hex;
mod level;
mod render;
mod reset;
mod turn;
mod ui;
mod undo;

use bevy::prelude::*;

use crate::components::MovePath;
use crate::level::LevelConfig;
use crate::turn::{GameSettings, Turn, TurnPhase, TurnState};

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: None,
            hovered_enemy: None,
            hovered_bomb: None,
            player_prev_hex: None,
        })
        .insert_resource(TurnState::Active(Turn::Player))
        .init_resource::<turn::PendingKills>()
        .init_resource::<LevelConfig>()
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_plugins(debug_ui::DebugUiPlugin)
        .add_plugins(undo::UndoPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(reset::ResetPlugin)
        .add_systems(
            Update,
            (check_animation_done, combat::resolve_combat, combat::tick_bombs).chain(),
        )
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scale = 0.9;
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection));
}

/// When all MovePath animations finish, advance to the next phase.
fn check_animation_done(mut turn: ResMut<TurnState>, animating: Query<(), With<MovePath>>) {
    let TurnState::Animating { next } = *turn else {
        return;
    };

    if animating.is_empty() {
        *turn = match next {
            TurnPhase::Turn(t) => TurnState::Active(t),
            TurnPhase::Combat(c) => TurnState::Combat(c),
        };
    }
}
