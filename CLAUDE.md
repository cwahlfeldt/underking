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

- **`src/main.rs`** - App entry point. Registers plugins (Tile, Player, Enemy), sets up Camera2d and `GameSettings` resource.
- **`src/hex.rs`** - Core hex grid math using cube coordinates (q, r, s with q+r+s=0). Includes `Hex` (coordinates, distance, pixel conversion), `Direction`, and `HexGrid<T>` (HashMap-backed hexagonal grid). Has comprehensive unit tests.
- **`src/tile.rs`** - `TilePlugin` spawns the hex grid as pickable mesh entities. Click-to-select updates `GameSettings.selected_hex`.
- **`src/player.rs`** - `PlayerPlugin` spawns player at a hex and moves to the selected hex each frame.
- **`src/enemy.rs`** - `EnemyPlugin` spawns an enemy entity.
- **`src/components/`** - Shared Bevy components (currently `Health`).
- **`src/consts/`** - Game constants (`HEX_SIZE`).
- **`src/util/`** - Utility functions (`hex_to_rgb` color parsing).

## Key Patterns

- Hex coordinates use cube coordinate system. Always construct via `Hex::new(q, r, s)` (asserts constraint) or `Hex::axial(q, r)` (derives s).
- `Hex::to_pixel` / `Hex::from_pixel` convert between hex coords and world-space for flat-topped layout.
- Game state flows through `GameSettings` resource (e.g., `selected_hex`).
- Entities are rendered as `Text2d` glyphs positioned on hex centers.
- Bevy observer pattern used for tile hover/click interactions.
