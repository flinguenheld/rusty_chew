use core::fmt::Write;
use heapless::String;

pub fn pins_to_str(left: &[u8; 4], right: &[u8; 4]) -> [String<50>; 4] {
    let mut rows: [String<50>; 4] = [String::new(), String::new(), String::new(), String::new()];

    // write!(&mut rows[0], "{:05b}   {:05b}\r\n", left[0], right[0]).ok();
    // write!(&mut rows[1], "{:05b}   {:05b}\r\n", left[1], right[1]).ok();
    // write!(&mut rows[2], "{:04b}     {:04b}\r\n", left[2], right[2]).ok();
    // write!(&mut rows[3], "   {:03b} {:03b}\r\n", left[3], right[3]).ok();

    write!(&mut rows[0], "{:08b}   {:08b}\r\n", left[0], right[0]).ok();
    write!(&mut rows[1], "{:08b}   {:08b}\r\n", left[1], right[1]).ok();
    write!(&mut rows[2], "{:08b}   {:08b}\r\n", left[2], right[2]).ok();
    write!(&mut rows[3], "{:08b}   {:08b}\r\n", left[3], right[3]).ok();
    rows
}

pub fn num_to_str(num: u8) -> String<1> {
    let mut l: String<1> = String::new();
    write!(&mut l, "{}", num).ok();
    l
}

pub fn line(ticks: u64) -> String<50> {
    let mut l: String<50> = String::new();

    let ms = ticks / 1_000;
    let seconds = ms / 1_000;
    let minutes = seconds / 60;
    let seconds = seconds % 60;

    write!(
        &mut l,
        "------------------ {:02}:{:02}:{} -> \r\n",
        minutes, seconds, ms
    )
    .ok();
    l
}
