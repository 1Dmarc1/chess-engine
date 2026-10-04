use crate::board::bitboard::Bitboard;
use crate::movement::attacks::generate_dynamic_diagonal_attacks;
use std::arch::x86_64::_pext_u64;

/// Precomputed lookup table mapping `(square, pext_index)` to bishop attack bitboards.
/// This is initialized during engine startup.
pub static mut BISHOP_ATTACK_TABLE: [[Bitboard; 1024]; 64] = [[Bitboard(0); 1024]; 64];

/// Precomputed blocker masks for bishop attack generation.
const BISHOP_MASKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = generate_bishop_mask(i as u8);
        i += 1;
    }
    table
};

/// Calculates the bishop attack bitboard for a square given the current board occupancy.
/// Does not differentiate between friendly and opponent pieces.
/// # Arguments
///- `square`: The 0-indexed square where the bishop is located.
/// - `total_occupancy`: Bitboard containing all pieces currently on the board.
#[inline(always)]
pub(crate) fn get_bishop_attacks(square: u8, total_occupancy: Bitboard) -> Bitboard {
    unsafe {
        let mask = BISHOP_MASKS[square as usize].0;
        let index = _pext_u64(total_occupancy.0, mask);
        BISHOP_ATTACK_TABLE[square as usize][index as usize]
    }
}

pub(crate) fn init_bishop_tables() {
    unsafe {
        for square in 0..64 {
            let mask = BISHOP_MASKS[square as usize].0;
            let mut occupancy = 0u64;
            loop {
                let attack = generate_dynamic_diagonal_attacks(square as u8, Bitboard(occupancy));
                let index = _pext_u64(occupancy, mask);
                BISHOP_ATTACK_TABLE[square as usize][index as usize] = attack;

                occupancy = occupancy.wrapping_sub(mask) & mask;
                if occupancy == 0 {
                    break;
                }
            }
        }
    }
}

const fn generate_bishop_mask(square: u8) -> Bitboard {
    let mut mask = 64;

    let rank = (square / 8) as i8;
    let file = (square % 8) as i8;

    // Up-Right
    let mut r = rank + 1;
    let mut f = file + 1;
    while r < 7 && f < 7 {
        mask |= 1u64 << (r * 8 + f);
        r += 1;
        f += 1;
    }

    // Up-Left
    let mut r = rank + 1;
    let mut f = file - 1;
    while r < 7 && f > 0 {
        mask |= 1u64 << (r * 8 + f);
        r += 1;
        f -= 1;
    }

    // Down-Right
    let mut r = rank - 1;
    let mut f = file + 1;
    while r > 0 && f < 7 {
        mask |= 1u64 << (r * 8 + f);
        r -= 1;
        f += 1;
    }

    // Down-Left
    let mut r = rank - 1;
    let mut f = file - 1;
    while r > 0 && f > 0 {
        mask |= 1u64 << (r * 8 + f);
        r -= 1;
        f -= 1;
    }

    Bitboard(mask)
}
