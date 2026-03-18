use bevy::prelude::*;

use crate::{
    components::Health,
    entities::player::Player,
    level::LevelConfig,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_health_ui, spawn_level_ui))
            .add_systems(Update, (update_health_ui, update_level_ui));
    }
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

#[derive(Component)]
struct LevelText;

fn spawn_level_ui(mut commands: Commands, level_config: Res<LevelConfig>) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(16.0),
            top: Val::Px(48.0),
            ..default()
        })
        .with_child((
            LevelText,
            Text::new(format!("Level: {}", level_config.current + 1)),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

fn update_level_ui(
    level_config: Res<LevelConfig>,
    mut text: Query<&mut Text, With<LevelText>>,
) {
    if !level_config.is_changed() {
        return;
    }
    let Ok(mut t) = text.single_mut() else { return };
    **t = format!("Level: {}", level_config.current + 1);
}
