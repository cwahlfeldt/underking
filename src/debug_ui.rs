use bevy::prelude::*;

use crate::{
    TurnState,
    components::{HexPosition, Stats},
    entities::{enemy::Enemy, player::Player},
    grid::TileData,
    hex::{HEX_SIZE, HexGrid},
    undo::UndoHistory,
};

const COORD_LABEL_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.3);
const COORD_FONT_SIZE: f32 = 11.0;
const COORD_INSET_FACTOR: f32 = 0.55;
const GAP: f32 = 1.5;

pub struct DebugUiPlugin;

impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DebugUiVisible(false))
            .add_systems(
                Startup,
                (
                    spawn_debug_panel,
                    spawn_coord_labels.after(crate::entities::tile::spawn_tiles),
                ),
            )
            .add_systems(Update, (toggle_debug, update_debug_panel));
    }
}

#[derive(Resource)]
pub struct DebugUiVisible(pub bool);

#[derive(Component)]
struct DebugPanel;

#[derive(Component)]
struct CoordLabel;

// --- Debug panel (UI overlay) ---

fn spawn_debug_panel(mut commands: Commands) {
    commands
        .spawn((
            DebugPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(10.0)),
                max_height: Val::Percent(90.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
        ))
        .with_child((
            Text::new(""),
            TextFont {
                font_size: 13.0,
                ..default()
            },
            TextColor(Color::srgb(0.0, 1.0, 0.4)),
        ));
}

// --- Coordinate labels (world-space Text2d on each hex) ---

fn spawn_coord_labels(mut commands: Commands, grid: Res<HexGrid<TileData>>) {
    let inset = (HEX_SIZE - GAP) * COORD_INSET_FACTOR;

    for pos in grid.positions() {
        let (cx, cy) = pos.to_pixel(HEX_SIZE);
        let h = pos;

        // q at top, r at bottom-left, s at bottom-right
        let labels: [(f32, f32, String); 3] = [
            (0.0, inset, format!("{}", h.q)),
            (-inset * 0.87, -inset * 0.5, format!("{}", h.r)),
            (inset * 0.87, -inset * 0.5, format!("{}", h.s)),
        ];

        for (lx, ly, text) in labels {
            commands.spawn((
                CoordLabel,
                Text2d::new(text),
                TextFont {
                    font_size: COORD_FONT_SIZE,
                    ..default()
                },
                TextColor(COORD_LABEL_COLOR),
                Transform::from_xyz(cx + lx, cy + ly, 2.0),
                Visibility::Hidden,
            ));
        }
    }
}

// --- Toggle everything on backtick ---

fn toggle_debug(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<DebugUiVisible>,
    mut panel_query: Query<&mut Visibility, With<DebugPanel>>,
    mut label_query: Query<&mut Visibility, (With<CoordLabel>, Without<DebugPanel>)>,
) {
    if !keys.just_pressed(KeyCode::Backquote) {
        return;
    }

    visible.0 = !visible.0;
    let vis = if visible.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    for mut v in &mut panel_query {
        *v = vis;
    }
    for mut v in &mut label_query {
        *v = vis;
    }
}

// --- Update debug panel text on turn change ---

fn update_debug_panel(
    visible: Res<DebugUiVisible>,
    turn: Res<TurnState>,
    grid: Res<HexGrid<TileData>>,
    history: Res<UndoHistory>,
    panel_query: Query<&Children, With<DebugPanel>>,
    mut text_query: Query<&mut Text>,
    player_query: Query<(Entity, &HexPosition, &Stats), With<Player>>,
    enemy_query: Query<(Entity, &HexPosition, &Stats), (With<Enemy>, Without<Player>)>,
) {
    if !visible.0 || (!turn.is_changed() && !history.is_changed()) {
        return;
    }

    let Ok(children) = panel_query.single() else {
        return;
    };
    let Some(&text_entity) = children.first() else {
        return;
    };
    let Ok(mut text) = text_query.get_mut(text_entity) else {
        return;
    };

    let mut lines = Vec::new();

    let turn_label = match *turn {
        TurnState::Active(crate::Turn::Player) => "PLAYER'S TURN",
        TurnState::Active(crate::Turn::Enemy) => "ENEMY TURN",
        TurnState::Animating { next } => match next {
            crate::TurnPhase::Turn(crate::Turn::Player) => "ANIMATING -> Player",
            crate::TurnPhase::Turn(crate::Turn::Enemy) => "ANIMATING -> Enemy",
            crate::TurnPhase::Combat(crate::CombatPhase::AfterPlayerMove) => "ANIMATING -> Combat (Player)",
            crate::TurnPhase::Combat(crate::CombatPhase::AfterEnemyMove) => "ANIMATING -> Combat (Enemy)",
        },
        TurnState::Combat(crate::CombatPhase::AfterPlayerMove) => "COMBAT (After Player)",
        TurnState::Combat(crate::CombatPhase::AfterEnemyMove) => "COMBAT (After Enemy)",
    };
    lines.push(format!("--- {turn_label} ---\n"));

    if let Ok((entity, pos, stats)) = player_query.single() {
        lines.push(format!(
            "Player ({:?})  @({},{})  mv:{}  atk:{}",
            entity, pos.0.q, pos.0.r, stats.move_range, stats.attack_range,
        ));
    }

    for (entity, pos, stats) in &enemy_query {
        lines.push(format!(
            "Enemy  ({:?})  @({},{})  mv:{}  atk:{}",
            entity, pos.0.q, pos.0.r, stats.move_range, stats.attack_range,
        ));
    }

    lines.push(String::new());
    lines.push("Grid:".to_string());

    for pos in grid.positions() {
        if let Some(tile) = grid.get(pos) {
            let mut flags = Vec::new();

            if !tile.traversable {
                flags.push("WALL".to_string());
            }
            if let Some(occ) = tile.occupant {
                flags.push(format!("occ:{occ:?}"));
            }
            if !tile.attack_ranges.is_empty() {
                let ids: Vec<String> = tile
                    .attack_ranges
                    .iter()
                    .map(|e| format!("{e:?}"))
                    .collect();
                flags.push(format!("atk:[{}]", ids.join(",")));
            }
            if !tile.move_ranges.is_empty() {
                let ids: Vec<String> = tile.move_ranges.iter().map(|e| format!("{e:?}")).collect();
                flags.push(format!("mov:[{}]", ids.join(",")));
            }

            if !flags.is_empty() {
                lines.push(format!("  ({},{})  {}", pos.q, pos.r, flags.join("  "),));
            }
        }
    }

    lines.push("\n[`] toggle".to_string());

    **text = lines.join("\n");
}
