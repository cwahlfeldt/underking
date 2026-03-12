use bevy::prelude::*;

use crate::{
    TurnState,
    components::{Dead, HexPosition, MovePath, RewindPath},
    entities::enemy::EnemyTurnQueue,
    grid::TileData,
    hex::{HEX_SIZE, Hex, HexGrid},
};

pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UndoHistory::default())
            .insert_resource(UndoAction::None)
            .insert_resource(RewindQueue::default())
            .insert_resource(TurnMoveOrder::default())
            .add_systems(Startup, spawn_undo_buttons)
            .add_systems(
                Update,
                (
                    handle_undo_redo_keys,
                    handle_undo_button,
                    handle_redo_button,
                    apply_undo_action,
                    process_rewind_queue,
                    update_button_colors,
                )
                    .chain(),
            );
    }
}

/// Signals an undo or redo action to be processed this frame.
#[derive(Resource, PartialEq, Eq)]
pub enum UndoAction {
    None,
    Undo,
    Redo,
}

/// Tracks the order entities moved during the current turn sequence.
/// Reset at the start of each player move. Used to build animation order for undo/redo.
#[derive(Resource, Default)]
pub struct TurnMoveOrder(pub Vec<Entity>);

/// A snapshot of all game state needed to restore a turn.
#[derive(Clone)]
pub struct GameSnapshot {
    pub turn_state: TurnState,
    pub grid_cells: Vec<(Hex, TileData)>,
    pub positions: Vec<(Entity, Hex)>,
    /// Order entities moved in this turn sequence (player first, then enemies).
    pub move_order: Vec<Entity>,
}

#[derive(Resource, Default)]
pub struct UndoHistory {
    pub undo_stack: Vec<GameSnapshot>,
    pub redo_stack: Vec<GameSnapshot>,
}

/// Queued rewind animations to play sequentially.
/// Each entry is one entity animating at a time.
#[derive(Resource, Default)]
struct RewindQueue {
    /// Each entry: (entity, from_pixel, to_pixel). Played one at a time.
    steps: Vec<(Entity, Vec2, Vec2)>,
    /// The entity currently animating.
    active: Option<Entity>,
    /// Entities with held RewindPath (speed=0) waiting for their turn or not needing animation.
    held: Vec<Entity>,
}

// --- UI Components ---

#[derive(Component)]
struct UndoButton;

#[derive(Component)]
struct RedoButton;

const BTN_COLOR: Color = Color::srgba(0.3, 0.3, 0.3, 0.9);
const BTN_COLOR_DISABLED: Color = Color::srgba(0.2, 0.2, 0.2, 0.5);
const BTN_COLOR_HOVER: Color = Color::srgba(0.45, 0.45, 0.45, 0.9);
const BTN_TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.9);
const BTN_TEXT_DISABLED: Color = Color::srgba(1.0, 1.0, 1.0, 0.3);

fn spawn_undo_buttons(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(15.0),
            right: Val::Px(15.0),
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    UndoButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(BTN_COLOR),
                ))
                .with_child((
                    Text::new("Undo"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(BTN_TEXT_COLOR),
                ));

            parent
                .spawn((
                    RedoButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(BTN_COLOR),
                ))
                .with_child((
                    Text::new("Redo"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(BTN_TEXT_COLOR),
                ));
        });
}

// --- Button interaction ---

fn handle_undo_button(
    query: Query<&Interaction, (Changed<Interaction>, With<UndoButton>)>,
    history: Res<UndoHistory>,
    mut action: ResMut<UndoAction>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed && !history.undo_stack.is_empty() {
            *action = UndoAction::Undo;
        }
    }
}

fn handle_redo_button(
    query: Query<&Interaction, (Changed<Interaction>, With<RedoButton>)>,
    history: Res<UndoHistory>,
    mut action: ResMut<UndoAction>,
) {
    for interaction in &query {
        if *interaction == Interaction::Pressed && !history.redo_stack.is_empty() {
            *action = UndoAction::Redo;
        }
    }
}

fn update_button_colors(
    history: Res<UndoHistory>,
    mut undo_query: Query<
        (&Interaction, &mut BackgroundColor, &Children),
        (With<UndoButton>, Without<RedoButton>),
    >,
    mut redo_query: Query<
        (&Interaction, &mut BackgroundColor, &Children),
        (With<RedoButton>, Without<UndoButton>),
    >,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, mut bg, children) in &mut undo_query {
        let enabled = !history.undo_stack.is_empty();
        *bg = if !enabled {
            BTN_COLOR_DISABLED.into()
        } else if *interaction == Interaction::Hovered {
            BTN_COLOR_HOVER.into()
        } else {
            BTN_COLOR.into()
        };
        if let Some(&child) = children.first() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                *tc = if enabled {
                    BTN_TEXT_COLOR.into()
                } else {
                    BTN_TEXT_DISABLED.into()
                };
            }
        }
    }

    for (interaction, mut bg, children) in &mut redo_query {
        let enabled = !history.redo_stack.is_empty();
        *bg = if !enabled {
            BTN_COLOR_DISABLED.into()
        } else if *interaction == Interaction::Hovered {
            BTN_COLOR_HOVER.into()
        } else {
            BTN_COLOR.into()
        };
        if let Some(&child) = children.first() {
            if let Ok(mut tc) = text_query.get_mut(child) {
                *tc = if enabled {
                    BTN_TEXT_COLOR.into()
                } else {
                    BTN_TEXT_DISABLED.into()
                };
            }
        }
    }
}

// --- Keyboard shortcuts ---

fn handle_undo_redo_keys(keys: Res<ButtonInput<KeyCode>>, mut action: ResMut<UndoAction>) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let ctrl = keys.pressed(KeyCode::ControlLeft)
        || keys.pressed(KeyCode::ControlRight)
        || keys.pressed(KeyCode::SuperLeft)
        || keys.pressed(KeyCode::SuperRight);

    if !ctrl {
        return;
    }

    if keys.just_pressed(KeyCode::KeyZ) && !shift {
        *action = UndoAction::Undo;
    } else if (keys.just_pressed(KeyCode::KeyZ) && shift) || keys.just_pressed(KeyCode::KeyY) {
        *action = UndoAction::Redo;
    }
}

// --- Core undo/redo logic ---

pub fn capture_snapshot(
    grid: &HexGrid<TileData>,
    turn: &TurnState,
    move_order: &TurnMoveOrder,
) -> GameSnapshot {
    let mut grid_cells = Vec::new();
    let mut positions = Vec::new();

    for (hex, data) in grid.iter() {
        grid_cells.push((hex, data.clone()));
        if let Some(entity) = data.occupant {
            positions.push((entity, hex));
        }
    }

    GameSnapshot {
        turn_state: *turn,
        grid_cells,
        positions,
        move_order: move_order.0.clone(),
    }
}

pub fn push_undo(history: &mut UndoHistory, snapshot: GameSnapshot) {
    history.undo_stack.push(snapshot);
    history.redo_stack.clear();
}

fn restore_snapshot(
    snapshot: &GameSnapshot,
    grid: &mut HexGrid<TileData>,
    turn: &mut TurnState,
    position_query: &mut Query<&mut HexPosition>,
) {
    for (hex, data) in &snapshot.grid_cells {
        grid.insert(*hex, data.clone());
    }

    for &(entity, hex) in &snapshot.positions {
        if let Ok(mut pos) = position_query.get_mut(entity) {
            pos.0 = hex;
        }
    }

    *turn = snapshot.turn_state;
}

fn apply_undo_action(
    mut commands: Commands,
    mut action: ResMut<UndoAction>,
    mut history: ResMut<UndoHistory>,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut queue: ResMut<EnemyTurnQueue>,
    mut rewind_queue: ResMut<RewindQueue>,
    mut move_order: ResMut<TurnMoveOrder>,
    mut position_query: Query<&mut HexPosition>,
    mut transform_query: Query<&mut Transform>,
    animating: Query<Entity, With<MovePath>>,
    rewinding: Query<Entity, With<RewindPath>>,
    dead_query: Query<Entity, With<Dead>>,
    enemy_query: Query<Entity, With<crate::entities::enemy::Enemy>>,
) {
    if *action == UndoAction::None {
        return;
    }

    let is_undo = *action == UndoAction::Undo;
    *action = UndoAction::None;

    let snapshot = if is_undo {
        let Some(snap) = history.undo_stack.pop() else {
            return;
        };
        let current = capture_snapshot(&grid, &turn, &move_order);
        history.redo_stack.push(current);
        snap
    } else {
        let Some(snap) = history.redo_stack.pop() else {
            return;
        };
        let current = capture_snapshot(&grid, &turn, &move_order);
        history.undo_stack.push(current);
        snap
    };

    // Cancel any in-progress animations
    for entity in &animating {
        commands.entity(entity).remove::<MovePath>();
    }
    for entity in &rewinding {
        commands.entity(entity).remove::<RewindPath>();
    }
    queue.0.clear();
    rewind_queue.steps.clear();
    rewind_queue.active = None;
    rewind_queue.held.clear();

    // Restore the move order from the snapshot
    move_order.0 = snapshot.move_order.clone();

    // Capture current visual positions BEFORE restoring snapshot
    let mut visual_positions: Vec<(Entity, Vec2)> = Vec::new();
    for &(entity, _) in &snapshot.positions {
        if let Ok(transform) = transform_query.get(entity) {
            visual_positions
                .push((entity, Vec2::new(transform.translation.x, transform.translation.y)));
        }
    }

    // Restore grid, hex positions, and turn state from snapshot
    restore_snapshot(&snapshot, &mut grid, &mut turn, &mut position_query);

    // Sync Dead state with snapshot occupancy.
    let snapshot_occupants: Vec<Entity> = grid
        .positions()
        .iter()
        .filter_map(|pos| grid.get(*pos).and_then(|t| t.occupant))
        .collect();

    // Revive dead entities that are occupants in the snapshot
    for entity in &dead_query {
        if snapshot_occupants.contains(&entity) {
            commands
                .entity(entity)
                .remove::<Dead>()
                .insert(Visibility::Inherited);
        }
    }

    // Kill alive entities that are NOT occupants in the snapshot
    // (they were killed in the state we're restoring to)
    for entity in enemy_query.iter() {
        if !snapshot_occupants.contains(&entity) && dead_query.get(entity).is_err() {
            commands
                .entity(entity)
                .insert((Dead, Visibility::Hidden));
        }
    }

    // Build per-entity animation entries
    let mut anim_map: Vec<(Entity, Vec2, Vec2)> = Vec::new();

    for &(entity, target_hex) in &snapshot.positions {
        let (to_x, to_y) = target_hex.to_pixel(HEX_SIZE);
        let to = Vec2::new(to_x, to_y);

        let from = visual_positions
            .iter()
            .find(|(e, _)| *e == entity)
            .map(|(_, pos)| *pos)
            .unwrap_or(to);

        // Hold transform at current visual position
        if let Ok(mut transform) = transform_query.get_mut(entity) {
            transform.translation.x = from.x;
            transform.translation.y = from.y;
        }

        // Insert holding RewindPath on ALL entities so sync_hex_to_transform skips them
        commands.entity(entity).insert(RewindPath {
            from,
            to: from,
            progress: 0.0,
            speed: 0.0,
        });

        if from.distance(to) < 1.0 {
            rewind_queue.held.push(entity);
        } else {
            anim_map.push((entity, from, to));
        }
    }

    // Build the animation sequence based on move_order.
    // For undo: reverse order (last mover rewinds first).
    // For redo: forward order (player first, then enemies in order).
    let ordered_entities: Vec<Entity> = if is_undo {
        // The snapshot we popped is the OLD state (before the moves happened).
        // The move_order stored in the CURRENT snapshot (pushed to redo) tells us
        // who moved. But we already pushed current to redo. Let's use the redo
        // stack's top entry's move_order.
        // Actually: the current snapshot (just pushed) has the move_order of the
        // turn we're undoing. That's what we want.
        let current_move_order = &history.redo_stack.last().unwrap().move_order;
        current_move_order.iter().rev().copied().collect()
    } else {
        // Redoing: the snapshot we popped is from a previous undo. The move_order
        // in the CURRENT snapshot (just pushed to undo) has the order of the state
        // we're leaving. The snapshot we're restoring TO has the move_order of how
        // it was originally played.
        // Actually: the snapshot we're restoring has the move_order from when it was
        // captured (at undo time). The undo stack's top (just pushed) has the
        // current move_order. We want the ORIGINAL forward order, which is stored
        // in the undo_stack top's move_order.
        let current_move_order = &history.undo_stack.last().unwrap().move_order;
        current_move_order.clone()
    };

    // Build steps in the determined order. Only include entities that actually need animation.
    for entity in &ordered_entities {
        if let Some(idx) = anim_map.iter().position(|(e, _, _)| e == entity) {
            rewind_queue.steps.push(anim_map[idx]);
        }
    }

    // Add any animated entities not in the move_order (shouldn't happen, but safety)
    for &entry in &anim_map {
        if !ordered_entities.contains(&entry.0) {
            rewind_queue.steps.push(entry);
        }
    }

    // Always return to player's turn after undo/redo so enemy AI doesn't re-trigger
    *turn = TurnState::Active(crate::Turn::Player);

    let label = if is_undo { "Undo" } else { "Redo" };
    info!(
        "{label} (undo: {}, redo: {})",
        history.undo_stack.len(),
        history.redo_stack.len()
    );
}

/// Processes the rewind queue one entity at a time.
fn process_rewind_queue(
    mut commands: Commands,
    mut rewind_queue: ResMut<RewindQueue>,
    mut rewind_query: Query<&mut RewindPath>,
) {
    // Nothing to do
    if rewind_queue.steps.is_empty() && rewind_queue.active.is_none() {
        // Clean up any remaining held entities
        for entity in rewind_queue.held.drain(..) {
            commands.entity(entity).remove::<RewindPath>();
        }
        return;
    }

    // Check if the active entity is still animating
    if let Some(active) = rewind_queue.active {
        let done = rewind_query
            .get(active)
            .map(|r| r.progress >= 1.0)
            .unwrap_or(true);
        if !done {
            return;
        }
        rewind_queue.active = None;
    }

    if rewind_queue.steps.is_empty() {
        // All steps done — clean up held entities
        for entity in rewind_queue.held.drain(..) {
            commands.entity(entity).remove::<RewindPath>();
        }
        return;
    }

    // Start the next step
    let (entity, from, to) = rewind_queue.steps.remove(0);
    if let Ok(mut rewind) = rewind_query.get_mut(entity) {
        rewind.from = from;
        rewind.to = to;
        rewind.progress = 0.0;
        rewind.speed = crate::render::MOVE_SPEED * 1.5;
        rewind_queue.active = Some(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid() -> HexGrid<TileData> {
        let mut grid = HexGrid::new(1);
        for pos in grid.positions() {
            grid.insert(pos, TileData::default());
        }
        grid
    }

    fn dummy_entities(count: usize) -> Vec<Entity> {
        let mut world = bevy::ecs::world::World::new();
        (0..count).map(|_| world.spawn_empty().id()).collect()
    }

    #[test]
    fn capture_snapshot_preserves_grid() {
        let mut grid = make_grid();
        let entities = dummy_entities(1);
        let e = entities[0];
        let origin = Hex::ORIGIN;

        grid.get_mut(origin).unwrap().occupant = Some(e);

        let turn = TurnState::Active(crate::Turn::Player);
        let move_order = TurnMoveOrder::default();
        let snap = capture_snapshot(&grid, &turn, &move_order);

        assert_eq!(snap.positions.len(), 1);
        assert_eq!(snap.positions[0], (e, origin));
        assert_eq!(snap.turn_state, turn);

        let cell = snap.grid_cells.iter().find(|(h, _)| *h == origin).unwrap();
        assert_eq!(cell.1.occupant, Some(e));
    }

    #[test]
    fn push_undo_clears_redo() {
        let grid = make_grid();
        let turn = TurnState::Active(crate::Turn::Player);
        let move_order = TurnMoveOrder::default();
        let snap = capture_snapshot(&grid, &turn, &move_order);

        let mut history = UndoHistory::default();
        history.redo_stack.push(snap.clone());
        assert_eq!(history.redo_stack.len(), 1);

        push_undo(&mut history, snap);
        assert_eq!(history.undo_stack.len(), 1);
        assert_eq!(history.redo_stack.len(), 0);
    }

    #[test]
    fn undo_stack_ordering() {
        let grid = make_grid();
        let turn_p = TurnState::Active(crate::Turn::Player);
        let turn_e = TurnState::Active(crate::Turn::Enemy);
        let move_order = TurnMoveOrder::default();

        let snap1 = capture_snapshot(&grid, &turn_p, &move_order);
        let snap2 = capture_snapshot(&grid, &turn_e, &move_order);

        let mut history = UndoHistory::default();
        push_undo(&mut history, snap1);
        push_undo(&mut history, snap2);

        assert_eq!(history.undo_stack.len(), 2);
        let popped = history.undo_stack.pop().unwrap();
        assert_eq!(popped.turn_state, turn_e);
        let popped = history.undo_stack.pop().unwrap();
        assert_eq!(popped.turn_state, turn_p);
    }
}
