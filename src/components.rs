use bevy::prelude::*;

use crate::hex::Hex;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexPosition(pub Hex);
