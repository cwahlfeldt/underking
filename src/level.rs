use bevy::prelude::*;

/// Defines enemy counts and terrain for a single level.
#[derive(Clone, Debug)]
pub struct Level {
    pub grunts: usize,
    pub archers: usize,
    pub warlocks: usize,
    pub bombers: usize,
    pub walls: usize,
    pub grid_radius: i32,
}

/// Resource holding the list of levels and the current level index.
#[derive(Resource)]
pub struct LevelConfig {
    pub levels: Vec<Level>,
    pub current: usize,
}

impl LevelConfig {
    pub fn current_level(&self) -> &Level {
        &self.levels[self.current]
    }

    /// Advance to the next level. Returns true if there is a next level.
    pub fn advance(&mut self) -> bool {
        if self.current + 1 < self.levels.len() {
            self.current += 1;
            true
        } else {
            false
        }
    }
}

impl Default for LevelConfig {
    fn default() -> Self {
        Self {
            levels: vec![
                Level {
                    grunts: 2,
                    archers: 0,
                    warlocks: 0,
                    bombers: 0,
                    walls: 5,
                    grid_radius: 4,
                },
                Level {
                    grunts: 2,
                    archers: 1,
                    warlocks: 0,
                    bombers: 0,
                    walls: 8,
                    grid_radius: 4,
                },
                Level {
                    grunts: 2,
                    archers: 1,
                    warlocks: 1,
                    bombers: 0,
                    walls: 10,
                    grid_radius: 4,
                },
                Level {
                    grunts: 3,
                    archers: 1,
                    warlocks: 1,
                    bombers: 1,
                    walls: 12,
                    grid_radius: 5,
                },
                Level {
                    grunts: 4,
                    archers: 2,
                    warlocks: 1,
                    bombers: 1,
                    walls: 15,
                    grid_radius: 5,
                },
            ],
            current: 0,
        }
    }
}
