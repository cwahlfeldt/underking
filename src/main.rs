mod combat;
mod components;
mod debug_ui;
mod entities;
mod grid;
mod hex;
mod render;
mod turn;
mod undo;

use bevy::prelude::*;

use crate::components::{Dead, Health, HexPosition, MovePath, Stats};
use crate::entities::enemy::Enemy;
use crate::entities::player::Player;
use crate::turn::{GameSettings, Turn, TurnPhase, TurnState};

fn main() {
    App::new()
        .insert_resource(GameSettings {
            selected_hex: None,
            hovered_enemy: None,
            player_prev_hex: None,
        })
        .insert_resource(TurnState::Active(Turn::Player))
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_plugins(render::RenderPlugin)
        .add_plugins(entities::tile::TilePlugin)
        .add_plugins(entities::player::PlayerPlugin)
        .add_plugins(entities::enemy::EnemyPlugin)
        .add_plugins(debug_ui::DebugUiPlugin)
        .add_plugins(undo::UndoPlugin)
        .add_systems(
            Update,
            (check_animation_done, combat::resolve_combat).chain(),
        )
        .add_systems(Startup, (setup, spawn_health_ui))
        .add_systems(Update, (update_health_ui, unlock_player_on_clear))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Component)]
struct HealthText;

fn spawn_health_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        })
        .with_child((
            HealthText,
            Text::new("HP: 3 / 3"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn update_health_ui(
    player: Query<&Health, With<Player>>,
    mut text: Query<&mut Text, With<HealthText>>,
) {
    let Ok(health) = player.single() else { return };
    let Ok(mut t) = text.single_mut() else { return };
    **t = format!("HP: {} / {}", health.current as i32, health.max as i32);
}

/// When all enemies are dead, set the player's move range to 0 (unlimited).
fn unlock_player_on_clear(
    enemies: Query<(), (With<Enemy>, Without<Dead>)>,
    mut player: Query<(Entity, &HexPosition, &mut Stats), With<Player>>,
    mut grid: ResMut<crate::hex::HexGrid<crate::grid::TileData>>,
) {
    if !enemies.is_empty() {
        return;
    }
    let Ok((entity, pos, mut stats)) = player.single_mut() else {
        return;
    };
    if stats.move_range == 0 {
        return;
    }
    stats.move_range = 0;
    crate::grid::clear_ranges(&mut grid, entity);
    crate::grid::update_ranges(&mut grid, pos.0, entity, &stats);
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
