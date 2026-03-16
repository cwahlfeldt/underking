mod combat;
mod components;
mod debug_ui;
mod entities;
mod grid;
mod hex;
mod render;
mod turn;
mod undo;

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::components::{Bomb, Dead, GameEntity, Health, HexPosition, MovePath, Stats};
use crate::entities::enemy::Enemy;
use crate::entities::player::Player;
use crate::turn::{GameSettings, PendingKills, Turn, TurnPhase, TurnState};

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
        .add_systems(Startup, (setup, spawn_health_ui, spawn_reset_button))
        .add_systems(Update, (update_health_ui, unlock_player_on_clear, handle_reset, tick_bombs))
        .run();
}

fn setup(mut commands: Commands) {
    let mut projection = OrthographicProjection::default_2d();
    projection.scale = 0.9;
    commands
        .spawn(Camera2d)
        .insert(Projection::Orthographic(projection));
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

#[derive(Component)]
struct ResetButton;

fn spawn_reset_button(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(16.0),
            top: Val::Px(16.0),
            ..default()
        })
        .with_child((
            ResetButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8)),
            children![(
                Text::new("Reset"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            )],
        ));
}

fn handle_reset(
    mut commands: Commands,
    interaction: Query<&Interaction, (Changed<Interaction>, With<ResetButton>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_entities: Query<Entity, With<GameEntity>>,
    mut turn: ResMut<TurnState>,
    mut game_settings: ResMut<GameSettings>,
    mut pending_kills: ResMut<PendingKills>,
    mut history: ResMut<crate::undo::UndoHistory>,
    mut move_order: ResMut<crate::undo::TurnMoveOrder>,
) {
    let clicked = interaction
        .iter()
        .any(|i| *i == Interaction::Pressed);
    let key_pressed = keyboard.just_pressed(KeyCode::KeyR);

    if !clicked && !key_pressed {
        return;
    }

    // Despawn all game entities
    for entity in &game_entities {
        commands.entity(entity).despawn();
    }

    // Reset resources
    *turn = TurnState::Active(Turn::Player);
    *game_settings = GameSettings {
        selected_hex: None,
        hovered_enemy: None,
        hovered_bomb: None,
        player_prev_hex: None,
    };
    pending_kills.0.clear();
    *history = crate::undo::UndoHistory::default();
    move_order.0.clear();

    // Remove the old HexGrid resource so spawn_tiles can re-create it
    commands.remove_resource::<crate::hex::HexGrid<crate::grid::TileData>>();

    // Re-run spawn systems as one-shot systems
    commands.queue(|world: &mut World| {
        let _ = world.run_system_once(crate::entities::tile::spawn_tiles);
        let _ = world.run_system_once(crate::entities::player::spawn_player);
        let _ = world.run_system_once(crate::entities::enemy::spawn_enemies);
    });
}

/// Tick bomb fuses at the start of each player turn. Explode when timer hits 0.
fn tick_bombs(
    mut commands: Commands,
    turn: Res<TurnState>,
    mut was_player_turn: Local<bool>,
    mut bomb_query: Query<(Entity, &mut Bomb, &HexPosition)>,
    mut health_query: Query<(Entity, &HexPosition, &mut Health)>,
    mut grid: ResMut<crate::hex::HexGrid<crate::grid::TileData>>,
) {
    let is_player_turn = *turn == TurnState::Active(Turn::Player);

    // Only tick on the transition INTO the player's turn
    if !is_player_turn || *was_player_turn {
        *was_player_turn = is_player_turn;
        return;
    }
    *was_player_turn = true;

    let mut explosions: Vec<(Entity, crate::hex::Hex, i32, f32)> = Vec::new();

    for (bomb_entity, mut bomb, bomb_pos) in &mut bomb_query {
        if bomb.turns_remaining == 0 {
            explosions.push((bomb_entity, bomb_pos.0, bomb.blast_radius, bomb.damage));
        } else {
            bomb.turns_remaining -= 1;
        }
    }

    for (bomb_entity, bomb_hex, radius, damage) in explosions {
        let blast_hexes = bomb_hex.spiral(radius);

        // Collect damage targets first, then apply
        let mut damaged: Vec<(Entity, crate::hex::Hex, f32)> = Vec::new();
        for (entity, pos, mut health) in &mut health_query {
            if blast_hexes.contains(&pos.0) {
                info!("Bomb at {:?} damages {:?} for {}", bomb_hex, entity, damage);
                health.current -= damage;
                if health.current <= 0.0 {
                    damaged.push((entity, pos.0, health.current));
                }
            }
        }

        // Kill entities that died from the blast
        for (entity, hex, _) in damaged {
            commands.entity(entity).insert((Dead, Visibility::Hidden));
            if let Some(tile) = grid.get_mut(hex) {
                if tile.occupant == Some(entity) {
                    tile.occupant = None;
                }
            }
            crate::grid::clear_ranges(&mut grid, entity);
        }

        commands.entity(bomb_entity).despawn();
    }
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
