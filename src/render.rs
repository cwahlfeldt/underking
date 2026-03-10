use bevy::prelude::*;

use crate::components::HexPosition;
use crate::hex::HEX_SIZE;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, sync_hex_to_transform);
    }
}

fn sync_hex_to_transform(mut query: Query<(&HexPosition, &mut Transform), Changed<HexPosition>>) {
    for (hex_pos, mut transform) in &mut query {
        let (x, y) = hex_pos.0.to_pixel(HEX_SIZE);
        transform.translation.x = x;
        transform.translation.y = y;
    }
}

pub fn hex_to_rgb(hex: &str) -> (f32, f32, f32) {
    if hex.len() != 7 || !hex.starts_with('#') {
        panic!("Invalid hex color: {hex}. Expected format: #RRGGBB");
    }

    let hex = hex.trim_start_matches("#");
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.0;

    (r, g, b)
}
