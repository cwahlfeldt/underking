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
