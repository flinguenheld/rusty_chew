use heapless::String;

pub fn pins_to_str(left: u8, right: u8, length: u8) -> String<50> {
    let mut aa: String<50> = String::new();

    aa.push_str(rename(left, length).as_str()).ok();
    aa.push_str("   ").ok();
    aa.push_str(rename(right, length).as_str()).ok();
    aa.push_str("\r\n").ok();

    aa
}

fn rename(mut pins: u8, length: u8) -> String<5> {
    let mut aa: String<5> = String::new();

    for _ in 0..length {
        match pins & 1 == 1 {
            true => aa.push('1').ok(),
            false => aa.push('0').ok(),
        };
        pins >>= 1;
    }

    aa
    // aa.chars().rev().collect()
}
