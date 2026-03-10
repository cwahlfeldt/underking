# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Underking is a hex-grid game built with Rust and Bevy 0.18. It uses a 2D camera with flat-topped hexagonal tiles, a player character, and enemies rendered as text glyphs.

## Build & Run

```bash
cargo run          # run the game
cargo build        # build only
cargo test         # run all tests (hex module has unit tests)
cargo test hex     # run hex-specific tests
```

Dynamic linking is enabled for Bevy in dev profile for faster compile times.

## Architecture

- **`src/main.rs`** - App entry point. Registers plugins, sets up Camera2d and `GameSettings` resource.
- **`src/hex.rs`** - Core hex module: `Hex` (cube coordinates), `Direction`, `HexGrid<T>` (HashMap-backed grid with A* pathfinding), `TileData` (per-cell state: occupant, attack/move ranges), and `HEX_SIZE` constant. Has comprehensive unit tests.
- **`src/components.rs`** - Shared Bevy components: `Health`, `HexPosition`.
- **`src/render.rs`** - `RenderPlugin` syncs `HexPosition` → `Transform` on change. Also has `hex_to_rgb` color utility.
- **`src/entities/`** - Entity plugins:
  - **`tile.rs`** - `TilePlugin` spawns hex grid as pickable mesh entities, inserts `HexGrid<TileData>` resource. Click-to-select updates `GameSettings.selected_hex`.
  - **`player.rs`** - `PlayerPlugin` spawns player, handles A* movement within move range, updates grid occupancy and ranges.
  - **`enemy.rs`** - `EnemyPlugin` spawns enemy, registers attack ranges on grid.

## Key Patterns

- `HexGrid<TileData>` is the central game state resource. All game logic reads/writes it.
- Each grid cell (`TileData`) tracks: tile entity, occupant, and which entities have attack/move range over it.
- Hex coordinates use cube coordinate system. Always construct via `Hex::new(q, r, s)` (asserts constraint) or `Hex::axial(q, r)` (derives s).
- `Hex::to_pixel` / `Hex::from_pixel` convert between hex coords and world-space for flat-topped layout.
- Game state flows through `GameSettings` resource (e.g., `selected_hex`).
- Entities are rendered as `Text2d` glyphs positioned on hex centers.
- Bevy observer pattern used for tile hover/click interactions.
