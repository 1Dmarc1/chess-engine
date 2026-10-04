#[inline(always)]
pub const fn square_to_rank_file(square: u8) -> (u8, u8) {
    let rank = square / 8;
    let file = square % 8;
    (rank, file)
}

#[inline(always)]
pub const fn rank_file_to_square(rank: u8, file: u8) -> u8 {
    rank * 8 + file
}

pub fn square_to_algebraic_notation(square: u8) -> Option<String> {
    let (r, f) = square_to_rank_file(square);
    let rank = r + 1;
    let file = match f {
        0 => 'a',
        1 => 'b',
        2 => 'c',
        3 => 'd',
        4 => 'e',
        5 => 'f',
        6 => 'g',
        7 => 'h',
        _ => return None,
    };

    Some(format!("{}{}", file, rank))
}

pub fn algebraic_notation_to_square(algebra: &str) -> Option<usize> {
    let bytes = algebra.as_bytes();
    if bytes.len() != 2 {
        return None;
    }
    let file = (bytes[0].to_ascii_lowercase() - b'a') as usize;
    let rank = (bytes[1] - b'1') as usize;
    Some(rank * 8 + file)
}
