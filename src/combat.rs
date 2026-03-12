use std::collections::HashSet;

use bevy::prelude::*;

use crate::{Turn, TurnState};

/// Tracks per-turn combat state: which enemies have already attacked this turn,
/// and whether counter-attack processing has run for the current enemy turn.
#[derive(Resource, Default)]
pub struct CombatState {
    pub enemy_counter_attacks_done: bool,
    pub attacked_enemies: HashSet<Entity>,
}

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CombatState>()
            .add_systems(Update, reset_combat_on_player_turn);
    }
}

/// Reset counter-attack tracking when the player turn begins.
fn reset_combat_on_player_turn(turn: Res<TurnState>, mut combat_state: ResMut<CombatState>) {
    if *turn == TurnState::Active(Turn::Player) && combat_state.enemy_counter_attacks_done {
        combat_state.enemy_counter_attacks_done = false;
        combat_state.attacked_enemies.clear();
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::world::World;
    use std::collections::HashSet;

    use bevy::prelude::Entity;

    use crate::{
        components::Stats,
        grid::{TileData, update_ranges},
        hex::{Hex, HexGrid},
    };

    fn make_grid() -> HexGrid<TileData> {
        let mut grid = HexGrid::new(3);
        for pos in grid.positions() {
            grid.insert(pos, TileData::default());
        }
        grid
    }

    fn dummy_entities(count: usize) -> Vec<Entity> {
        let mut world = World::new();
        (0..count).map(|_| world.spawn_empty().id()).collect()
    }

    // Mirrors the counter-attack check in move_enemies: finds enemies whose
    // attack range covers the player's tile.
    fn enemies_attacking(
        grid: &HexGrid<TileData>,
        player_hex: Hex,
        enemy_set: &HashSet<Entity>,
    ) -> Vec<Entity> {
        grid.get(player_hex)
            .map(|t| {
                t.attack_ranges
                    .iter()
                    .copied()
                    .filter(|e| enemy_set.contains(e))
                    .collect()
            })
            .unwrap_or_default()
    }

    // Mirrors the adjacent-enemy check in handle_player_move.
    fn adjacent_enemy(
        grid: &HexGrid<TileData>,
        target: Hex,
        enemy_set: &HashSet<Entity>,
    ) -> Option<(Entity, Hex)> {
        grid.neighbors(target).into_iter().find_map(|n| {
            grid.get(n)
                .and_then(|t| t.occupant)
                .filter(|occ| enemy_set.contains(occ))
                .map(|occ| (occ, n))
        })
    }

    #[test]
    fn counter_attack_when_player_on_enemy_attack_tile() {
        let mut grid = make_grid();
        let e = dummy_entities(2);
        let (player, enemy) = (e[0], e[1]);

        let player_hex = Hex::ORIGIN;
        let enemy_hex = Hex::axial(1, 0); // adjacent → attack range covers ORIGIN

        grid.get_mut(player_hex).unwrap().occupant = Some(player);
        grid.get_mut(enemy_hex).unwrap().occupant = Some(enemy);
        update_ranges(&mut grid, enemy_hex, enemy, &Stats { move_range: 1, attack_range: 1 });

        let enemy_set: HashSet<Entity> = [enemy].into();
        let attackers = enemies_attacking(&grid, player_hex, &enemy_set);
        assert_eq!(attackers, vec![enemy]);
    }

    #[test]
    fn no_counter_attack_when_player_out_of_range() {
        let mut grid = make_grid();
        let e = dummy_entities(2);
        let (player, enemy) = (e[0], e[1]);

        let player_hex = Hex::ORIGIN;
        let enemy_hex = Hex::axial(3, 0); // 3 away, attack_range=1 can't reach ORIGIN

        grid.get_mut(enemy_hex).unwrap().occupant = Some(enemy);
        update_ranges(&mut grid, enemy_hex, enemy, &Stats { move_range: 1, attack_range: 1 });

        let enemy_set: HashSet<Entity> = [enemy].into();
        let attackers = enemies_attacking(&grid, player_hex, &enemy_set);
        assert!(attackers.is_empty());
    }

    #[test]
    fn attack_move_finds_enemy_adjacent_to_target() {
        let mut grid = make_grid();
        let e = dummy_entities(2);
        let (player, enemy) = (e[0], e[1]);

        let player_hex = Hex::ORIGIN;
        let target_hex = Hex::axial(1, 0); // player will move here
        let enemy_hex = Hex::axial(2, 0); // enemy is adjacent to target

        grid.get_mut(player_hex).unwrap().occupant = Some(player);
        grid.get_mut(enemy_hex).unwrap().occupant = Some(enemy);

        let enemy_set: HashSet<Entity> = [enemy].into();
        let result = adjacent_enemy(&grid, target_hex, &enemy_set);
        assert_eq!(result, Some((enemy, enemy_hex)));
    }

    #[test]
    fn no_attack_move_when_no_adjacent_enemy() {
        let mut grid = make_grid();
        let e = dummy_entities(1);
        let player = e[0];

        grid.get_mut(Hex::ORIGIN).unwrap().occupant = Some(player);
        let target_hex = Hex::axial(1, 0);

        let enemy_set: HashSet<Entity> = HashSet::new();
        let result = adjacent_enemy(&grid, target_hex, &enemy_set);
        assert!(result.is_none());
    }

    #[test]
    fn multiple_enemies_only_adjacent_one_detected() {
        let mut grid = make_grid();
        let e = dummy_entities(3);
        let (player, enemy_a, enemy_b) = (e[0], e[1], e[2]);

        let player_hex = Hex::ORIGIN;
        let target_hex = Hex::axial(1, 0);
        // enemy_a is adjacent to target, enemy_b is not
        let enemy_a_hex = Hex::axial(2, 0);
        let enemy_b_hex = Hex::axial(-2, 0);

        grid.get_mut(player_hex).unwrap().occupant = Some(player);
        grid.get_mut(enemy_a_hex).unwrap().occupant = Some(enemy_a);
        grid.get_mut(enemy_b_hex).unwrap().occupant = Some(enemy_b);

        let enemy_set: HashSet<Entity> = [enemy_a, enemy_b].into();
        let result = adjacent_enemy(&grid, target_hex, &enemy_set);
        assert_eq!(result, Some((enemy_a, enemy_a_hex)));
    }

    #[test]
    fn two_enemies_both_in_range_both_counter_attack() {
        let mut grid = make_grid();
        let e = dummy_entities(3);
        let (player, enemy_a, enemy_b) = (e[0], e[1], e[2]);

        let player_hex = Hex::ORIGIN;
        // Both enemies adjacent to player
        let ea_hex = Hex::axial(1, 0);
        let eb_hex = Hex::axial(-1, 0);

        grid.get_mut(player_hex).unwrap().occupant = Some(player);
        grid.get_mut(ea_hex).unwrap().occupant = Some(enemy_a);
        grid.get_mut(eb_hex).unwrap().occupant = Some(enemy_b);
        update_ranges(&mut grid, ea_hex, enemy_a, &Stats { move_range: 1, attack_range: 1 });
        update_ranges(&mut grid, eb_hex, enemy_b, &Stats { move_range: 1, attack_range: 1 });

        let enemy_set: HashSet<Entity> = [enemy_a, enemy_b].into();
        let mut attackers = enemies_attacking(&grid, player_hex, &enemy_set);
        attackers.sort();
        let mut expected = vec![enemy_a, enemy_b];
        expected.sort();
        assert_eq!(attackers, expected);
    }
}
