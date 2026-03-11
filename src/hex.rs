use bevy::prelude::Resource;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::ops::{Add, Neg, Sub};

pub const HEX_SIZE: f32 = 40.0;

/// Cube coordinate on a hex grid.
///
/// Invariant: `q + r + s == 0`. All constructors enforce this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
    pub s: i32,
}

impl Hex {
    /// Create from cube coordinates. Panics if `q + r + s != 0`.
    pub fn new(q: i32, r: i32, s: i32) -> Self {
        assert!(
            q + r + s == 0,
            "cube constraint violated: q + r + s must equal 0"
        );
        Self { q, r, s }
    }

    /// Create from axial coordinates (q, r). `s` is derived.
    pub fn axial(q: i32, r: i32) -> Self {
        Self { q, r, s: -q - r }
    }

    /// The origin hex.
    pub const ORIGIN: Self = Self { q: 0, r: 0, s: 0 };

    /// Manhattan distance from the origin.
    pub fn length(self) -> i32 {
        (self.q.abs() + self.r.abs() + self.s.abs()) / 2
    }

    /// Manhattan distance to another hex.
    pub fn distance(self, other: Hex) -> i32 {
        (self - other).length()
    }

    /// The six neighbors of this hex, in directional order starting from E.
    pub fn neighbors(self) -> [Hex; 6] {
        DIRECTIONS.map(|d| self + d)
    }

    /// The neighbor in a specific direction.
    pub fn neighbor(self, dir: Direction) -> Hex {
        self + DIRECTIONS[dir as usize]
    }

    /// All hexes exactly `radius` steps away (a ring).
    /// Returns an empty vec for radius 0.
    pub fn ring(self, radius: i32) -> Vec<Hex> {
        if radius <= 0 {
            return vec![];
        }
        let mut results = Vec::with_capacity(6 * radius as usize);
        // Start at the hex `radius` steps in the SW direction from center.
        let mut current = self + DIRECTIONS[Direction::SW as usize].scale(radius);
        for dir_idx in 0..6 {
            for _ in 0..radius {
                results.push(current);
                current = current + DIRECTIONS[dir_idx];
            }
        }
        results
    }

    /// All hexes within `radius` steps (filled disk), including center.
    pub fn spiral(self, radius: i32) -> Vec<Hex> {
        let mut results = vec![self];
        for r in 1..=radius {
            results.extend(self.ring(r));
        }
        results
    }

    /// Scale all components by a scalar.
    pub fn scale(self, factor: i32) -> Hex {
        Hex::new(self.q * factor, self.r * factor, self.s * factor)
    }

    /// Convert to flat-topped hex world-space center point.
    pub fn to_pixel(self, size: f32) -> (f32, f32) {
        let q = self.q as f32;
        let r = self.r as f32;
        let x = size * (3.0 / 2.0 * q);
        let y = size * (SQRT_3 / 2.0 * q + SQRT_3 * r);
        (x, y)
    }

    /// Convert a world-space point back to the nearest hex (flat-topped).
    pub fn from_pixel(x: f32, y: f32, size: f32) -> Hex {
        let q = (2.0 / 3.0 * x) / size;
        let r = (-1.0 / 3.0 * x + SQRT_3 / 3.0 * y) / size;
        cube_round(q, r)
    }

    /// The six corner vertices of this hex in world-space (flat-topped).
    pub fn corners(self, size: f32) -> [(f32, f32); 6] {
        let (cx, cy) = self.to_pixel(size);
        let mut corners = [(0.0, 0.0); 6];
        for i in 0..6 {
            let angle_deg = 60.0 * i as f32;
            let angle_rad = angle_deg.to_radians();
            corners[i] = (cx + size * angle_rad.cos(), cy + size * angle_rad.sin());
        }
        corners
    }
}

const SQRT_3: f32 = 1.732_050_8;

/// Round fractional cube coordinates to the nearest hex.
fn cube_round(fq: f32, fr: f32) -> Hex {
    let fs = -fq - fr;
    let mut q = fq.round();
    let mut r = fr.round();
    let s = fs.round();

    let q_diff = (q - fq).abs();
    let r_diff = (r - fr).abs();
    let s_diff = (s - fs).abs();

    if q_diff > r_diff && q_diff > s_diff {
        q = -r - s;
    } else if r_diff > s_diff {
        r = -q - s;
    }

    Hex::new(q as i32, r as i32, (-q - r) as i32)
}

impl Add for Hex {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            q: self.q + rhs.q,
            r: self.r + rhs.r,
            s: self.s + rhs.s,
        }
    }
}

impl Sub for Hex {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            q: self.q - rhs.q,
            r: self.r - rhs.r,
            s: self.s - rhs.s,
        }
    }
}

impl Neg for Hex {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            q: -self.q,
            r: -self.r,
            s: -self.s,
        }
    }
}

// ---------------------------------------------------------------------------
// Directions
// ---------------------------------------------------------------------------

/// The six hex directions for a flat-topped hex grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Direction {
    E = 0,
    NE = 1,
    NW = 2,
    W = 3,
    SW = 4,
    SE = 5,
}

impl Direction {
    pub const ALL: [Direction; 6] = [
        Direction::E,
        Direction::NE,
        Direction::NW,
        Direction::W,
        Direction::SW,
        Direction::SE,
    ];

    /// The opposite direction.
    pub fn opposite(self) -> Direction {
        Direction::ALL[(self as usize + 3) % 6]
    }
}

/// Unit vectors for each direction in cube coordinates (flat-topped).
const DIRECTIONS: [Hex; 6] = [
    Hex { q: 1, r: 0, s: -1 }, // E
    Hex { q: 1, r: -1, s: 0 }, // NE
    Hex { q: 0, r: -1, s: 1 }, // NW
    Hex { q: -1, r: 0, s: 1 }, // W
    Hex { q: -1, r: 1, s: 0 }, // SW
    Hex { q: 0, r: 1, s: -1 }, // SE
];

// ---------------------------------------------------------------------------
// HexGrid
// ---------------------------------------------------------------------------

/// A hex grid arranged in a true hexagonal shape.
///
/// The grid has a `radius` measured from center to edge in hex steps.
/// A radius-0 grid is a single hex; radius-1 has 7 hexes, etc.
///
/// Each cell can hold an optional value of type `T`.
#[derive(Debug, Clone, Resource)]
pub struct HexGrid<T> {
    pub radius: i32,
    cells: HashMap<Hex, T>,
}

impl<T> HexGrid<T> {
    /// Create a new hexagonal grid. Every cell starts empty.
    pub fn new(radius: i32) -> Self {
        assert!(radius >= 0);
        Self {
            radius,
            cells: HashMap::with_capacity(hex_count(radius)),
        }
    }

    /// Total number of cells in the grid.
    pub fn len(&self) -> usize {
        hex_count(self.radius)
    }

    pub fn is_empty(&self) -> bool {
        self.radius < 0
    }

    /// Whether a coordinate is within the grid bounds.
    pub fn contains(&self, hex: Hex) -> bool {
        hex.distance(Hex::ORIGIN) <= self.radius
    }

    /// Get a reference to the value at `hex`, if present.
    pub fn get(&self, hex: Hex) -> Option<&T> {
        self.cells.get(&hex)
    }

    /// Get a mutable reference to the value at `hex`, if present.
    pub fn get_mut(&mut self, hex: Hex) -> Option<&mut T> {
        self.cells.get_mut(&hex)
    }

    /// Insert a value at `hex`. Returns the previous value if the cell was occupied.
    /// Returns `None` without inserting if `hex` is out of bounds.
    pub fn insert(&mut self, hex: Hex, value: T) -> Option<T> {
        if !self.contains(hex) {
            return None;
        }
        self.cells.insert(hex, value)
    }

    /// Remove and return the value at `hex`.
    pub fn remove(&mut self, hex: Hex) -> Option<T> {
        self.cells.remove(&hex)
    }

    /// Iterate over all occupied cells as `(Hex, &T)`.
    pub fn iter(&self) -> impl Iterator<Item = (Hex, &T)> {
        self.cells.iter().map(|(&h, v)| (h, v))
    }

    /// Iterate over all valid positions in the grid (occupied or not).
    pub fn positions(&self) -> Vec<Hex> {
        Hex::ORIGIN.spiral(self.radius)
    }

    /// Neighbors of `hex` that are within grid bounds.
    pub fn neighbors(&self, hex: Hex) -> Vec<Hex> {
        hex.neighbors()
            .into_iter()
            .filter(|&n| self.contains(n))
            .collect()
    }

    /// A* pathfinding from `start` to `goal`.
    ///
    /// `passable` returns true if a hex can be traversed. Both `start` and
    /// `goal` are assumed passable regardless.
    ///
    /// Returns the path as a vec of hexes from `start` to `goal` (inclusive),
    /// or `None` if no path exists.
    pub fn astar(&self, start: Hex, goal: Hex, passable: impl Fn(Hex) -> bool) -> Option<Vec<Hex>> {
        if !self.contains(start) || !self.contains(goal) {
            return None;
        }
        if start == goal {
            return Some(vec![start]);
        }

        let mut open = BinaryHeap::new();
        let mut came_from: HashMap<Hex, Hex> = HashMap::new();
        let mut g_score: HashMap<Hex, i32> = HashMap::new();

        g_score.insert(start, 0);
        open.push(AstarNode {
            hex: start,
            f: start.distance(goal),
        });

        while let Some(current) = open.pop() {
            if current.hex == goal {
                let mut path = vec![goal];
                let mut node = goal;
                while let Some(&prev) = came_from.get(&node) {
                    path.push(prev);
                    node = prev;
                }
                path.reverse();
                return Some(path);
            }

            let current_g = g_score[&current.hex];

            for neighbor in self.neighbors(current.hex) {
                if neighbor != goal && !passable(neighbor) {
                    continue;
                }

                let tentative_g = current_g + 1;
                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current.hex);
                    g_score.insert(neighbor, tentative_g);
                    open.push(AstarNode {
                        hex: neighbor,
                        f: tentative_g + neighbor.distance(goal),
                    });
                }
            }
        }

        None
    }
}

struct AstarNode {
    hex: Hex,
    f: i32,
}

impl Eq for AstarNode {}

impl PartialEq for AstarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f == other.f
    }
}

impl Ord for AstarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f) // reversed for min-heap
    }
}

impl PartialOrd for AstarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Number of hexes in a hexagonal grid of the given radius.
/// Formula: 3*r^2 + 3*r + 1
pub fn hex_count(radius: i32) -> usize {
    let r = radius as i64;
    (3 * r * r + 3 * r + 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_constraint() {
        let h = Hex::axial(3, -1);
        assert_eq!(h.s, -2);
        assert_eq!(h.q + h.r + h.s, 0);
    }

    #[test]
    #[should_panic]
    fn cube_constraint_violation() {
        Hex::new(1, 1, 1);
    }

    #[test]
    fn distance_and_length() {
        let a = Hex::axial(0, 0);
        let b = Hex::axial(3, -1);
        assert_eq!(a.distance(b), 3);
        assert_eq!(b.length(), 3);
    }

    #[test]
    fn neighbors_count() {
        let h = Hex::ORIGIN;
        assert_eq!(h.neighbors().len(), 6);
        for n in h.neighbors() {
            assert_eq!(h.distance(n), 1);
        }
    }

    #[test]
    fn ring_sizes() {
        let center = Hex::ORIGIN;
        assert_eq!(center.ring(0).len(), 0);
        assert_eq!(center.ring(1).len(), 6);
        assert_eq!(center.ring(2).len(), 12);
        assert_eq!(center.ring(3).len(), 18);
    }

    #[test]
    fn spiral_size() {
        let center = Hex::ORIGIN;
        assert_eq!(center.spiral(0).len(), 1);
        assert_eq!(center.spiral(1).len(), 7);
        assert_eq!(center.spiral(2).len(), 19);
    }

    #[test]
    fn hex_count_formula() {
        assert_eq!(hex_count(0), 1);
        assert_eq!(hex_count(1), 7);
        assert_eq!(hex_count(2), 19);
        assert_eq!(hex_count(3), 37);
    }

    #[test]
    fn pixel_roundtrip() {
        let size = 16.0;
        for hex in Hex::ORIGIN.spiral(3) {
            let (px, py) = hex.to_pixel(size);
            let recovered = Hex::from_pixel(px, py, size);
            assert_eq!(hex, recovered, "roundtrip failed for {hex:?}");
        }
    }

    #[test]
    fn grid_bounds() {
        let grid: HexGrid<()> = HexGrid::new(2);
        assert_eq!(grid.len(), 19);
        assert!(grid.contains(Hex::ORIGIN));
        assert!(grid.contains(Hex::axial(2, 0)));
        assert!(!grid.contains(Hex::axial(3, 0)));
    }

    #[test]
    fn grid_insert_get() {
        let mut grid = HexGrid::new(2);
        assert!(grid.insert(Hex::ORIGIN, 42).is_none());
        assert_eq!(grid.get(Hex::ORIGIN), Some(&42));
        assert!(grid.insert(Hex::axial(5, 0), 99).is_none()); // out of bounds
        assert_eq!(grid.get(Hex::axial(5, 0)), None);
    }

    #[test]
    fn grid_neighbors_at_edge() {
        let grid: HexGrid<()> = HexGrid::new(1);
        // edge hex has fewer in-bounds neighbors than center
        let edge_neighbors = grid.neighbors(Hex::axial(1, 0));
        assert!(edge_neighbors.len() < 6);
        // center always has all 6 (for radius >= 1)
        let center_neighbors = grid.neighbors(Hex::ORIGIN);
        assert_eq!(center_neighbors.len(), 6);
    }

    #[test]
    fn direction_opposite() {
        assert_eq!(Direction::E.opposite(), Direction::W);
        assert_eq!(Direction::NE.opposite(), Direction::SW);
        assert_eq!(Direction::NW.opposite(), Direction::SE);
    }

    #[test]
    fn astar_straight_path() {
        let grid: HexGrid<()> = HexGrid::new(3);
        let start = Hex::axial(-2, 0);
        let goal = Hex::axial(2, 0);
        let path = grid.astar(start, goal, |_| true).unwrap();
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
        assert_eq!(path.len(), 5); // distance is 4, path has 5 nodes
    }

    #[test]
    fn astar_same_start_and_goal() {
        let grid: HexGrid<()> = HexGrid::new(2);
        let path = grid.astar(Hex::ORIGIN, Hex::ORIGIN, |_| true).unwrap();
        assert_eq!(path, vec![Hex::ORIGIN]);
    }

    #[test]
    fn astar_blocked() {
        let grid: HexGrid<()> = HexGrid::new(2);
        let start = Hex::axial(-2, 0);
        let goal = Hex::axial(2, 0);
        // block everything — no path possible
        let path = grid.astar(start, goal, |_| false);
        assert!(path.is_none());
    }

    #[test]
    fn astar_around_obstacle() {
        let grid: HexGrid<()> = HexGrid::new(3);
        let start = Hex::axial(-1, 0);
        let goal = Hex::axial(1, 0);
        let wall = Hex::ORIGIN;
        let path = grid.astar(start, goal, |h| h != wall).unwrap();
        assert!(!path.contains(&wall));
        assert_eq!(path.first(), Some(&start));
        assert_eq!(path.last(), Some(&goal));
    }

    #[test]
    fn astar_out_of_bounds() {
        let grid: HexGrid<()> = HexGrid::new(1);
        let result = grid.astar(Hex::ORIGIN, Hex::axial(5, 0), |_| true);
        assert!(result.is_none());
    }
}
