use crate::board::bitboard::Bitboard;
use crate::move_gen::attacks::generate_dynamic_straight_attacks;
use std::arch::x86_64::_pext_u64;

/// Precomputed lookup table mapping `(square, pext_index)` to rook attack bitboards.
/// This is initialized during engine startup.
static mut ROOK_ATTACK_TABLE: [[Bitboard; 4096]; 64] = [[Bitboard(0); 4096]; 64];

/// Precomputed blocker masks for rook attack generation.
const ROOK_MASKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = generate_rook_masks(i as u8);
        i += 1;
    }
    table
};

/// Calculates the rook attack bitboard for a square given the current board occupancy.
/// Does not differentiate between friendly and opponent pieces.
/// # Arguments
///- `square`: The 0-indexed square where the rook is located.
/// - `total_occupancy`: Bitboard containing all pieces currently on the board.
#[inline(always)]
pub(crate) fn get_rook_attacks(square: u8, total_occupancy: Bitboard) -> Bitboard {
    unsafe {
        let mask = ROOK_MASKS[square as usize].0;
        let index = _pext_u64(total_occupancy.0, mask);
        ROOK_ATTACK_TABLE[square as usize][index as usize]
    }
}

pub(crate) fn init_rook_tables() {
    unsafe {
        for square in 0..64 {
            let mask = ROOK_MASKS[square as usize].0;

            // Start with an empty board
            let mut occupancy = 0u64;

            // Loop through every possible arrangement of blockers on the mask
            loop {
                // Get the true attack
                let attack = generate_dynamic_straight_attacks(square as u8, Bitboard(occupancy));
                let index = _pext_u64(occupancy, mask); // Compress to index
                ROOK_ATTACK_TABLE[square as usize][index as usize] = attack; // Store the attack in the lookup table

                occupancy = occupancy.wrapping_sub(mask) & mask; // Get the next combination

                // When we wrap back around to 0, we've tested all combinations
                if occupancy == 0 {
                    break;
                }
            }
        }
    }
}

const fn generate_rook_masks(square: u8) -> Bitboard {
    let mut mask = 0u64;
    let rank = (square / 8) as i8;
    let file = (square % 8) as i8;

    // Up
    let mut r = rank + 1;
    while r < 7 {
        mask |= 1u64 << (r * 8 + file);
        r += 1;
    }

    // Down
    let mut r = rank - 1;
    while r > 0 {
        mask |= 1u64 << (r * 8 + file);
        r -= 1;
    }

    // Right
    let mut f = file + 1;
    while f < 7 {
        mask |= 1u64 << (rank * 8 + f);
        f += 1;
    }

    // Left
    let mut f = file - 1;
    while f > 0 {
        mask |= 1u64 << (rank * 8 + f);
        f -= 1;
    }

    Bitboard(mask)
}
