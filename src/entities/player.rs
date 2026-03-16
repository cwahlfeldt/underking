use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::{
    components::{
        AttackAnimation, AttackPhase, GameEntity, Health, HexPosition, MovePath, Stats, ZOffset,
    },
    grid::{TileData, is_passable, move_entity, update_ranges},
    hex::{HEX_SIZE, Hex, HexGrid, iso_z_from_y},
    render::MOVE_SPEED,
    turn::{CombatPhase, GameSettings, Turn, TurnPhase, TurnState},
};

const SPRITE_FRAME_SIZE: UVec2 = UVec2::new(128, 128);
const SPRITE_COLUMNS: u32 = 15;
const SPRITE_ROWS: u32 = 8;
const SPRITE_FPS: f32 = 16.0;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player.after(super::tile::spawn_tiles))
            .add_systems(
                Update,
                (
                    render_player,
                    update_player_anim_state,
                    update_facing_from_movement,
                    animate_player_sprite,
                    animate_attack,
                    handle_player_move,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
pub struct Player;

/// Tracks which sprite sheet row to use for facing direction.
/// Row indices map to 8 directions in the sprite sheet.
#[derive(Component)]
pub struct FacingDirection(pub u32);

/// Which animation the player is currently playing.
#[derive(Component, PartialEq, Eq, Clone, Copy)]
pub enum PlayerAnimState {
    Idle,
    Walking,
    Attacking,
}

/// Holds preloaded sprite sheet handles so we can swap between them.
#[derive(Component)]
struct PlayerSpriteSheets {
    idle_texture: Handle<Image>,
    idle_layout: Handle<TextureAtlasLayout>,
    walk_texture: Handle<Image>,
    walk_layout: Handle<TextureAtlasLayout>,
    attack_texture: Handle<Image>,
    attack_layout: Handle<TextureAtlasLayout>,
}

pub fn spawn_player(mut commands: Commands, mut grid: ResMut<HexGrid<TileData>>) {
    let mut rng = rand::rng();
    let mut candidates: Vec<Hex> = grid
        .positions()
        .into_iter()
        .filter(|&h| is_passable(&grid, h))
        .collect();
    candidates.shuffle(&mut rng);
    let start_coord = candidates[0];

    let stats = Stats {
        move_range: 1,
        attack_range: 1,
    };

    let entity = commands
        .spawn((
            Player,
            GameEntity,
            Health {
                current: 3.0,
                max: 3.0,
            },
            HexPosition(start_coord),
            Stats {
                move_range: stats.move_range,
                attack_range: stats.attack_range,
            },
        ))
        .id();

    if let Some(tile) = grid.get_mut(start_coord) {
        tile.occupant = Some(entity);
    }

    update_ranges(&mut grid, start_coord, entity, &stats);
}

/// Timer driving sprite sheet animation.
#[derive(Component)]
struct SpriteAnimationTimer(Timer);

fn render_player(
    mut commands: Commands,
    query: Query<Entity, Added<Player>>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    for entity in &query {
        let idle_texture: Handle<Image> = asset_server.load("sprites/knight/Idle.png");
        let walk_texture: Handle<Image> = asset_server.load("sprites/knight/Walk.png");
        let attack_texture: Handle<Image> = asset_server.load("sprites/knight/MeleeRun.png");

        let make_layout = || {
            TextureAtlasLayout::from_grid(
                SPRITE_FRAME_SIZE,
                SPRITE_COLUMNS,
                SPRITE_ROWS,
                None,
                None,
            )
        };
        let idle_layout = texture_atlas_layouts.add(make_layout());
        let walk_layout = texture_atlas_layouts.add(make_layout());
        let attack_layout = texture_atlas_layouts.add(make_layout());

        commands.entity(entity).insert((
            Transform::default(),
            Sprite {
                image: idle_texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: idle_layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            bevy::sprite::Anchor(Vec2::new(0.0, -0.25)),
            ZOffset(1.0),
            FacingDirection(0),
            PlayerAnimState::Idle,
            PlayerSpriteSheets {
                idle_texture,
                idle_layout,
                walk_texture,
                walk_layout,
                attack_texture,
                attack_layout,
            },
            SpriteAnimationTimer(Timer::from_seconds(1.0 / SPRITE_FPS, TimerMode::Repeating)),
        ));
    }
}

fn animate_player_sprite(
    time: Res<Time>,
    mut query: Query<(&mut SpriteAnimationTimer, &mut Sprite, &FacingDirection), With<Player>>,
) {
    for (mut timer, mut sprite, facing) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                let row_start = (facing.0 * SPRITE_COLUMNS) as usize;
                let current_col = atlas.index.saturating_sub(row_start);
                let next_col = (current_col + 1) % SPRITE_COLUMNS as usize;
                atlas.index = row_start + next_col;
            }
        }
    }
}

/// Maps a movement direction (pixel-space delta) to a sprite sheet row index.
/// Sheet rows: 0=E, 1=SE, 2=S, 3=SW, 4=W, 5=NW, 6=N, 7=NE
fn angle_to_sprite_row(dx: f32, dy: f32) -> u32 {
    // Undo the iso Y squish so we get the true hex-grid angle
    let true_dy = dy / crate::hex::ISO_Y_SCALE;
    let angle = true_dy.atan2(dx).to_degrees();
    // Normalize to 0..360
    let angle = if angle < 0.0 { angle + 360.0 } else { angle };
    // atan2 sectors: 0=E, 1=NE, 2=N, 3=NW, 4=W, 5=SW, 6=S, 7=SE
    let sector = ((angle + 22.5) / 45.0) as u32 % 8;
    match sector {
        0 => 0, // E
        1 => 7, // NE
        2 => 6, // N
        3 => 5, // NW
        4 => 4, // W
        5 => 3, // SW
        6 => 2, // S
        7 => 1, // SE
        _ => 0,
    }
}

/// Swap between Idle and Walk sprite sheets based on movement state.
fn update_player_anim_state(
    mut query: Query<
        (
            &mut PlayerAnimState,
            &mut Sprite,
            &FacingDirection,
            &PlayerSpriteSheets,
            Option<&MovePath>,
            Option<&AttackAnimation>,
        ),
        With<Player>,
    >,
) {
    for (mut anim_state, mut sprite, facing, sheets, move_path, attack_anim) in &mut query {
        let desired = if attack_anim.is_some() {
            PlayerAnimState::Attacking
        } else if move_path.is_some() {
            PlayerAnimState::Walking
        } else {
            PlayerAnimState::Idle
        };

        if *anim_state != desired {
            let (texture, layout) = match desired {
                PlayerAnimState::Idle => (sheets.idle_texture.clone(), sheets.idle_layout.clone()),
                PlayerAnimState::Walking => {
                    (sheets.walk_texture.clone(), sheets.walk_layout.clone())
                }
                PlayerAnimState::Attacking => {
                    (sheets.attack_texture.clone(), sheets.attack_layout.clone())
                }
            };

            sprite.image = texture;
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.layout = layout;
                // Reset to first frame of current facing row
                atlas.index = (facing.0 * SPRITE_COLUMNS) as usize;
            }
            *anim_state = desired;
        }
    }
}

/// Update facing direction based on current movement path.
fn update_facing_from_movement(
    mut query: Query<(&MovePath, &mut FacingDirection, &mut Sprite), With<Player>>,
) {
    for (path, mut facing, mut sprite) in &mut query {
        let from = path.waypoints[path.current_index];
        let to_idx = (path.current_index + 1).min(path.waypoints.len() - 1);
        let to = path.waypoints[to_idx];
        let dir = to - from;
        if dir.length_squared() > 0.0 {
            let new_row = angle_to_sprite_row(dir.x, dir.y);
            if facing.0 != new_row {
                // Switch row, keep same column offset
                let old_row_start = (facing.0 * SPRITE_COLUMNS) as usize;
                let col = sprite
                    .texture_atlas
                    .as_ref()
                    .map(|a| a.index.saturating_sub(old_row_start))
                    .unwrap_or(0);
                facing.0 = new_row;
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = (new_row * SPRITE_COLUMNS) as usize + col;
                }
            }
        }
    }
}

/// Animate the attack lunge: forward to enemy, then back to home.
pub fn animate_attack(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &mut Transform,
            &mut AttackAnimation,
            &mut FacingDirection,
            &mut Sprite,
            Option<&ZOffset>,
        ),
        With<Player>,
    >,
) {
    for (entity, mut transform, mut attack, mut facing, mut sprite, z_offset) in &mut query {
        let z_off = z_offset.map(|z| z.0).unwrap_or(0.0);

        // Set facing toward the target
        let dir = attack.target - attack.home;
        if dir.length_squared() > 0.0 {
            let new_row = angle_to_sprite_row(dir.x, dir.y);
            if facing.0 != new_row {
                let old_row_start = (facing.0 * SPRITE_COLUMNS) as usize;
                let col = sprite
                    .texture_atlas
                    .as_ref()
                    .map(|a| a.index.saturating_sub(old_row_start))
                    .unwrap_or(0);
                facing.0 = new_row;
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = (new_row * SPRITE_COLUMNS) as usize + col;
                }
            }
        }

        let (from, to) = match attack.phase {
            AttackPhase::LungeForward => (attack.home, attack.target),
            AttackPhase::LungeBack => (attack.target, attack.home),
        };

        let distance = from.distance(to);
        if distance > 0.0 {
            attack.progress += time.delta_secs() * attack.speed / distance;
        } else {
            attack.progress = 1.0;
        }

        if attack.progress >= 1.0 {
            match attack.phase {
                AttackPhase::LungeForward => {
                    // Arrived at enemy — switch to lunge back
                    transform.translation.x = to.x;
                    transform.translation.y = to.y;
                    transform.translation.z = iso_z_from_y(to.y) + z_off;
                    attack.phase = AttackPhase::LungeBack;
                    attack.progress = 0.0;
                }
                AttackPhase::LungeBack => {
                    // Back home — done. Remove component.
                    transform.translation.x = to.x;
                    transform.translation.y = to.y;
                    transform.translation.z = iso_z_from_y(to.y) + z_off;
                    commands.entity(entity).remove::<AttackAnimation>();
                }
            }
        } else {
            let t = attack.progress;
            let pos = from.lerp(to, t);
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
            transform.translation.z = iso_z_from_y(pos.y) + z_off;
        }
    }
}

fn handle_player_move(
    mut commands: Commands,
    mut grid: ResMut<HexGrid<TileData>>,
    mut turn: ResMut<TurnState>,
    mut game_settings: ResMut<GameSettings>,
    mut history: ResMut<crate::undo::UndoHistory>,
    mut move_order: ResMut<crate::undo::TurnMoveOrder>,
    mut query: Query<(Entity, &mut HexPosition, &Stats), With<Player>>,
) {
    if *turn != TurnState::Active(Turn::Player) {
        return;
    }

    let Ok((entity, mut hex_pos, stats)) = query.single_mut() else {
        return;
    };

    let Some(target) = game_settings.selected_hex else {
        return;
    };
    if hex_pos.0 == target {
        return;
    }

    let can_move = grid
        .get(target)
        .map(|t| {
            t.traversable
                && t.occupant.is_none()
                && (stats.move_range == 0 || t.move_ranges.contains(&entity))
        })
        .unwrap_or(false);

    if !can_move {
        return;
    }

    let path = grid.astar(hex_pos.0, target, |h| is_passable(&grid, h));

    if let Some(path) = path {
        let snapshot = crate::undo::capture_snapshot(&grid, &turn, &move_order);
        crate::undo::push_undo(&mut history, snapshot);

        move_order.0.clear();
        move_order.0.push(entity);

        game_settings.selected_hex = None;
        game_settings.player_prev_hex = Some(hex_pos.0);
        move_entity(
            &mut commands,
            &mut grid,
            &mut turn,
            entity,
            &mut hex_pos,
            &path,
            stats,
            MOVE_SPEED,
            TurnPhase::Combat(CombatPhase::AfterPlayerMove),
        );
    }
}
