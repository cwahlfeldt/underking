# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Underking is a hex-grid game built with Rust and Bevy 0.18. It uses a 2D camera with flat-topped hexagonal tiles, a player character, and enemies rendered as text glyphs.

## Build & Run

```bash
cargo run          # run the game
cargo build        # build only
cargo test         # run all tests (29 tests across hex, grid, render)
cargo test hex     # run hex-specific tests
cargo test grid    # run grid-specific tests
```

Dynamic linking is enabled for Bevy in dev profile for faster compile times.

## Architecture

- **`src/main.rs`** - App entry point. Registers plugins, sets up Camera2d, `GameSettings` resource, and `TurnState`. Contains `check_animation_done` system that advances turns when animations finish.
- **`src/hex.rs`** - Pure hex math: `Hex` (cube coordinates), `Direction`, `HexGrid<T>` (HashMap-backed grid with A* pathfinding), and `HEX_SIZE` constant. 17 unit tests.
- **`src/grid.rs`** - Central game state module: `TileData` (per-cell state), shared helpers (`is_passable`, `update_ranges`, `clear_ranges`, `move_entity`). 9 unit tests.
- **`src/components.rs`** - Shared Bevy components: `Health`, `HexPosition`, `Stats` (move/attack range), `MovePath` (waypoint animation).
- **`src/render.rs`** - `RenderPlugin` syncs `HexPosition` → `Transform` on change. Animates `MovePath` waypoints with easing. 3 unit tests.
- **`src/debug_ui.rs`** - Self-contained debug overlay. Backtick toggles a UI panel (turn state, entity positions, grid data) and world-space hex coordinate labels (q/r/s at respective corners).
- **`src/entities/`** - Entity plugins:
  - **`tile.rs`** - `TilePlugin` spawns hex grid as pickable mesh entities, inserts `HexGrid<TileData>` resource. Hover highlights enemy attack ranges. Per-tile materials with `RestMaterial` for reliable restoration.
  - **`player.rs`** - `PlayerPlugin` spawns player with `Stats`, handles A* movement via `move_entity` helper.
  - **`enemy.rs`** - `EnemyPlugin` spawns enemies with `Stats`, sequential AI movement via `EnemyTurnQueue`.

## Key Patterns

- `HexGrid<TileData>` is the central game state resource. All game logic reads/writes it.
- Each grid cell (`TileData`) tracks: tile entity, occupant, traversability, and which entities have attack/move range over it.
- Shared grid helpers in `grid.rs` (`is_passable`, `update_ranges`, `clear_ranges`, `move_entity`) keep entity modules thin.
- `Stats` component on each entity defines move_range and attack_range (used by grid helpers to compute ranges).
- Turn system: `TurnState::Active(Turn::Player|Enemy)` for input, `TurnState::Animating { next }` during movement. `check_animation_done` advances when all `MovePath` components are consumed.
- Hex coordinates use cube coordinate system. Always construct via `Hex::new(q, r, s)` (asserts q+r+s=0) or `Hex::axial(q, r)` (derives s).
- `Hex::to_pixel` / `Hex::from_pixel` convert between hex coords and world-space for flat-topped layout.
- Game state flows through `GameSettings` resource (`selected_hex`, `hovered_enemy`).
- Entities are rendered as `Text2d` glyphs positioned on hex centers.
- Bevy observer pattern used for tile hover/click interactions.
- Each tile has its own `ColorMaterial` handle; `RestMaterial` component stores the original for restoration after highlights.
