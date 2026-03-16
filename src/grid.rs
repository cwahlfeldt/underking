use bevy::prelude::*;

use crate::{
    components::{HexPosition, MovePath, Stats},
    hex::{HEX_SIZE, Hex, HexGrid},
    turn::{TurnPhase, TurnState},
};

/// Per-cell game state stored in the hex grid.
#[derive(Debug, Clone)]
pub struct TileData {
    pub tile_entity: Option<Entity>,
    pub occupant: Option<Entity>,
    pub traversable: bool,
    pub attack_ranges: Vec<Entity>,
    pub move_ranges: Vec<Entity>,
}

impl Default for TileData {
    fn default() -> Self {
        Self {
            tile_entity: None,
            occupant: None,
            traversable: true,
            attack_ranges: Vec::new(),
            move_ranges: Vec::new(),
        }
    }
}

/// Returns true if the tile at `hex` is a valid movement target.
pub fn is_passable(grid: &HexGrid<TileData>, hex: Hex) -> bool {
    grid.get(hex)
        .map(|t| t.traversable && t.occupant.is_none())
        .unwrap_or(false)
}

/// Update the move/attack range overlays for an entity at `pos`.
pub fn update_ranges(grid: &mut HexGrid<TileData>, pos: Hex, entity: Entity, stats: &Stats) {
    if stats.move_range > 0 {
        let move_hexes: Vec<Hex> = pos
            .spiral(stats.move_range)
            .into_iter()
            .filter(|&h| h != pos && grid.contains(h))
            .collect();

        for hex in move_hexes {
            if let Some(tile) = grid.get_mut(hex) {
                tile.move_ranges.push(entity);
            }
        }
    }

    let attack_hexes: Vec<Hex> = pos
        .spiral(stats.attack_range)
        .into_iter()
        .filter(|&h| h != pos && grid.contains(h))
        .collect();

    for hex in attack_hexes {
        if let Some(tile) = grid.get_mut(hex) {
            tile.attack_ranges.push(entity);
        }
    }
}

/// Clear all move/attack range entries for `entity` across the entire grid.
pub fn clear_ranges(grid: &mut HexGrid<TileData>, entity: Entity) {
    let positions: Vec<Hex> = grid.positions();
    for pos in positions {
        if let Some(tile) = grid.get_mut(pos) {
            tile.move_ranges.retain(|&e| e != entity);
            tile.attack_ranges.retain(|&e| e != entity);
        }
    }
}

/// Move an entity from its current hex to `destination` along `path`.
/// Updates grid occupancy, ranges, and inserts a `MovePath` for animation.
/// Sets `turn` to `Animating { next }`.
pub fn move_entity(
    commands: &mut Commands,
    grid: &mut HexGrid<TileData>,
    turn: &mut TurnState,
    entity: Entity,
    hex_pos: &mut HexPosition,
    path: &[Hex],
    stats: &Stats,
    speed: f32,
    next_phase: TurnPhase,
) {
    if path.len() < 2 {
        return;
    }

    let old_pos = hex_pos.0;
    let destination = *path.last().unwrap();

    hex_pos.0 = destination;

    if let Some(tile) = grid.get_mut(old_pos) {
        tile.occupant = None;
    }
    if let Some(tile) = grid.get_mut(destination) {
        tile.occupant = Some(entity);
    }

    clear_ranges(grid, entity);
    update_ranges(grid, destination, entity, stats);

    let waypoints: Vec<Vec2> = path
        .iter()
        .map(|h| {
            let (x, y) = h.to_iso_pixel(HEX_SIZE);
            Vec2::new(x, y)
        })
        .collect();

    commands.entity(entity).insert(MovePath {
        waypoints,
        current_index: 0,
        progress: 0.0,
        speed,
    });

    *turn = TurnState::Animating { next: next_phase };
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::World;

    fn make_grid() -> HexGrid<TileData> {
        let mut grid = HexGrid::new(2);
        for pos in grid.positions() {
            grid.insert(pos, TileData::default());
        }
        grid
    }

    fn dummy_entities(count: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..count).map(|_| world.spawn_empty().id()).collect()
    }

    #[test]
    fn tile_data_default_is_traversable() {
        let td = TileData::default();
        assert!(td.traversable);
        assert!(td.occupant.is_none());
        assert!(td.tile_entity.is_none());
        assert!(td.attack_ranges.is_empty());
        assert!(td.move_ranges.is_empty());
    }

    #[test]
    fn is_passable_traversable_empty() {
        let grid = make_grid();
        assert!(is_passable(&grid, Hex::ORIGIN));
    }

    #[test]
    fn is_passable_wall() {
        let mut grid = make_grid();
        grid.get_mut(Hex::ORIGIN).unwrap().traversable = false;
        assert!(!is_passable(&grid, Hex::ORIGIN));
    }

    #[test]
    fn is_passable_occupied() {
        let mut grid = make_grid();
        let entities = dummy_entities(1);
        grid.get_mut(Hex::ORIGIN).unwrap().occupant = Some(entities[0]);
        assert!(!is_passable(&grid, Hex::ORIGIN));
    }

    #[test]
    fn is_passable_out_of_bounds() {
        let grid = make_grid();
        assert!(!is_passable(&grid, Hex::axial(10, 0)));
    }

    #[test]
    fn update_ranges_attack_only_when_move_zero() {
        let mut grid = make_grid();
        let entities = dummy_entities(1);
        let e = entities[0];
        let stats = Stats {
            move_range: 0,
            attack_range: 1,
        };
        update_ranges(&mut grid, Hex::ORIGIN, e, &stats);

        for pos in grid.positions() {
            assert!(
                !grid.get(pos).unwrap().move_ranges.contains(&e),
                "move_range should be empty when stats.move_range == 0"
            );
        }

        for neighbor in Hex::ORIGIN.neighbors() {
            if grid.contains(neighbor) {
                assert!(grid.get(neighbor).unwrap().attack_ranges.contains(&e));
            }
        }

        assert!(!grid.get(Hex::ORIGIN).unwrap().attack_ranges.contains(&e));
    }

    #[test]
    fn update_ranges_move_and_attack() {
        let mut grid = make_grid();
        let entities = dummy_entities(1);
        let e = entities[0];
        let stats = Stats {
            move_range: 1,
            attack_range: 1,
        };
        update_ranges(&mut grid, Hex::ORIGIN, e, &stats);

        for neighbor in Hex::ORIGIN.neighbors() {
            if grid.contains(neighbor) {
                assert!(grid.get(neighbor).unwrap().move_ranges.contains(&e));
                assert!(grid.get(neighbor).unwrap().attack_ranges.contains(&e));
            }
        }
    }

    #[test]
    fn clear_ranges_removes_entity() {
        let mut grid = make_grid();
        let entities = dummy_entities(1);
        let e = entities[0];
        let stats = Stats {
            move_range: 1,
            attack_range: 2,
        };
        update_ranges(&mut grid, Hex::ORIGIN, e, &stats);
        clear_ranges(&mut grid, e);

        for pos in grid.positions() {
            let tile = grid.get(pos).unwrap();
            assert!(!tile.move_ranges.contains(&e));
            assert!(!tile.attack_ranges.contains(&e));
        }
    }

    #[test]
    fn clear_ranges_preserves_other_entities() {
        let mut grid = make_grid();
        let entities = dummy_entities(2);
        let e1 = entities[0];
        let e2 = entities[1];
        let stats = Stats {
            move_range: 1,
            attack_range: 1,
        };
        update_ranges(&mut grid, Hex::ORIGIN, e1, &stats);
        update_ranges(&mut grid, Hex::axial(1, 0), e2, &stats);

        clear_ranges(&mut grid, e1);

        let has_e2 = grid.positions().iter().any(|&pos| {
            let tile = grid.get(pos).unwrap();
            tile.move_ranges.contains(&e2) || tile.attack_ranges.contains(&e2)
        });
        assert!(has_e2);
    }
}
