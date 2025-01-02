use super::timer::ChewTimer;

// 00  01  02  03  04    |    05  06  07  08  09
// 10  11  12  13  14    |    15  16  17  18  19
// 20  21  22  23        |        24  25  26  27
//         28  29  30    |    31  32  33

pub type Matrix = [u32; 34];

pub fn up_matrix(
    mut rows: [u8; 4],
    side: char,
    chew_timer: &ChewTimer,
    previous: &Matrix,
    current: &mut Matrix,
) {
    let indexes = match side {
        'l' => [(0, 4), (10, 14), (20, 23), (28, 30)],
        _ => [(5, 9), (15, 19), (24, 27), (31, 33)],
    };

    for (row, index) in rows.iter_mut().zip(indexes.iter()) {
        for i in (index.0..=index.1).rev() {
            if *row & 1 == 1 {
                match previous[i] > 0 {
                    true => current[i] = previous[i],
                    false => current[i] = chew_timer.ticks,
                }
            } else {
                current[i] = 0;
            }
            *row >>= 1;
        }
    }
}
