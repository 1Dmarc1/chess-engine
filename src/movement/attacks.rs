use crate::board::bitboard::Bitboard;
use crate::util;

/// Contains all possible attacks a knight could ever do at a given position.
pub(crate) const KNIGHT_ATTACKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = Bitboard(generate_knight_attacks(i as u8));
        i += 1;
    }
    table
};

/// Contains all possible attacks a white pawn could ever do at a given position.
pub(crate) const WHITE_PAWN_ATTACKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = generate_pawn_attacks(i as u8, true);
        i += 1;
    }
    table
};

/// Contains all possible attacks a black pawn could ever do at a given position.
pub(crate) const BLACK_PAWN_ATTACKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = generate_pawn_attacks(i as u8, false);
        i += 1;
    }
    table
};

/// Contains all possible attacks a king could ever do at a given position.
pub const KING_ATTACKS: [Bitboard; 64] = {
    let mut table = [Bitboard(0); 64];
    let mut i = 0;
    while i < 64 {
        table[i] = generate_king_attacks(i as u8);
        i += 1;
    }
    table
};

/// Generates diagonal attacks inclusive the first occupied square.
pub(crate) fn generate_dynamic_diagonal_attacks(square: u8, occupancy: Bitboard) -> Bitboard {
    let mut bb = Bitboard(0);
    let (rank, file) = util::square_to_rank_file(square);

    // Diagonal left up
    let mut curr_rank = rank as i8 + 1;
    let mut curr_file = file as i8 - 1;
    while curr_rank < 8 && curr_file >= 0 {
        bb.set(util::rank_file_to_square(curr_rank as u8, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, curr_file as u8)) {
            break;
        }
        curr_rank += 1;
        curr_file -= 1;
    }

    // Diagonal right up
    let mut curr_rank = rank as i8 + 1;
    let mut curr_file = file as i8 + 1;
    while curr_rank < 8 && curr_file < 8 {
        bb.set(util::rank_file_to_square(curr_rank as u8, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, curr_file as u8)) {
            break;
        }
        curr_rank += 1;
        curr_file += 1;
    }

    // Diagonal left down
    let mut curr_rank = rank as i8 - 1;
    let mut curr_file = file as i8 - 1;
    while curr_rank >= 0 && curr_file >= 0 {
        bb.set(util::rank_file_to_square(curr_rank as u8, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, curr_file as u8)) {
            break;
        }
        curr_rank -= 1;
        curr_file -= 1;
    }

    // Diagonal right down
    let mut curr_rank = rank as i8 - 1;
    let mut curr_file = file as i8 + 1;
    while curr_rank >= 0 && curr_file < 8 {
        bb.set(util::rank_file_to_square(curr_rank as u8, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, curr_file as u8)) {
            break;
        }
        curr_rank -= 1;
        curr_file += 1;
    }

    bb
}

/// Generates straight attacks inclusive the first occupied square.
pub(crate) fn generate_dynamic_straight_attacks(square: u8, occupancy: Bitboard) -> Bitboard {
    let mut bb = Bitboard(0);
    let (rank, file) = util::square_to_rank_file(square);

    // Vertical up
    let mut curr_rank = rank as i8 + 1;
    while curr_rank < 8 {
        bb.set(util::rank_file_to_square(curr_rank as u8, file));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, file)) {
            break;
        }
        curr_rank += 1;
    }

    // Vertical down
    curr_rank = rank as i8 - 1;
    while curr_rank >= 0 {
        bb.set(util::rank_file_to_square(curr_rank as u8, file));
        if occupancy.is_set(util::rank_file_to_square(curr_rank as u8, file)) {
            break;
        }
        curr_rank -= 1;
    }

    // Vertical left
    let mut curr_file = file as i8 - 1;
    while curr_file >= 0 {
        bb.set(util::rank_file_to_square(rank, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(rank, curr_file as u8)) {
            break;
        }
        curr_file -= 1;
    }

    // Vertical right
    curr_file = file as i8 + 1;
    while curr_file < 8 {
        bb.set(util::rank_file_to_square(rank, curr_file as u8));
        if occupancy.is_set(util::rank_file_to_square(rank, curr_file as u8)) {
            break;
        }
        curr_file += 1;
    }

    bb
}

const fn generate_knight_attacks(square: u8) -> u64 {
    let rank = (square / 8) as i8;
    let file = (square % 8) as i8;
    let mut bb = 0u64;

    const KNIGHT_OFFSETS: [(i8, i8); 8] = [
        (2, 1),
        (2, -1),
        (-2, 1),
        (-2, -1),
        (1, 2),
        (1, -2),
        (-1, 2),
        (-1, -2),
    ];

    let mut i = 0;
    while i < 8 {
        let new_rank = rank + KNIGHT_OFFSETS[i].0;
        let new_file = file + KNIGHT_OFFSETS[i].1;

        if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
            let target_square = (new_rank * 8 + new_file) as u64;
            bb |= 1u64 << target_square;
        }
        i += 1;
    }
    bb
}

const fn generate_pawn_attacks(square: u8, is_white: bool) -> Bitboard {
    let mut bb = Bitboard(0);
    let (rank, file) = util::square_to_rank_file(square);

    if is_white{
        if rank < 7 {
            if file < 7 {
                bb.set(util::rank_file_to_square(rank + 1, file + 1)); // Up right
            }
            if file > 0 {
                bb.set(util::rank_file_to_square(rank + 1, file - 1)); // Up left
            }
        }
    }
    else{
        if rank > 0{
            if file < 7 {
                bb.set(util::rank_file_to_square(rank - 1, file + 1)); // Down right
            }
            if file > 0 {
                bb.set(util::rank_file_to_square(rank - 1, file - 1)); // Down left
            }
        }
    }
    
    bb
}

const fn generate_king_attacks(square: u8) -> Bitboard {
    let mut bb = Bitboard(0);
    let (r, f) = util::square_to_rank_file(square);
    let rank = r as i8;
    let file = f as i8;

    const KING_OFFSETS: [(i8, i8); 8] = [
        (1, 0),   // Up
        (-1, 0),  // Down
        (0, 1),   // Right
        (0, -1),  // Left
        (1, 1),   // Up-Right
        (1, -1),  // Up-Left
        (-1, 1),  // Down-Right
        (-1, -1), // Down-Left
    ];

    let mut i = 0;
    while i < 8 {
        let new_rank = rank + KING_OFFSETS[i].0;
        let new_file = file + KING_OFFSETS[i].1;
        if new_rank >= 0 && new_rank < 8 && new_file >= 0 && new_file < 8 {
            let target_square = (new_rank * 8 + new_file) as u8;
            bb.set(target_square);
        }
        i += 1;
    }

    bb
}
