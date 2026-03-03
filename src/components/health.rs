use bevy::prelude::*;

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}
