use smart_leds::RGB8;

pub const RED: [u8; 3] = [250, 0, 0];
pub const GREEN: [u8; 3] = [0, 250, 0];
pub const BLUE: [u8; 3] = [0, 0, 250];

pub fn color(rgb: [u8; 3], dim_percent: u16) -> RGB8 {
    let p = dim_percent.min(100);
    let [r, g, b] = rgb.map(|c| (c as u16 * p / 100) as u8);
    RGB8::new(r, g, b)
}
