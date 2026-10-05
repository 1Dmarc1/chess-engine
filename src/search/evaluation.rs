use crate::types::{Move, MoveType, piece};

const PIECE_VALUES: [i32; 6] = [
    100,    // Pawn
    320,    // Knight
    330,    // Bishop
    500,    // Rook
    900,    // Queen 
    20_000, // King
];

const PROMOTION_BONUSES: [i32; 16] = {
    let mut table = [0; 16];
    table[MoveType::QueenPromotion as usize] = 900_000;
    table[MoveType::QueenPromoCapture as usize] = 900_000;
    table[MoveType::RookPromotion as usize] = 500_000;
    table[MoveType::RookPromoCapture as usize] = 500_000;
    table[MoveType::BishopPromotion as usize] = 300_000;
    table[MoveType::BishopPromoCapture as usize] = 300_000;
    table[MoveType::KnightPromotion as usize] = 300_000;
    table[MoveType::KnightPromoCapture as usize] = 300_000;
    table
};

/// MVV_LVA scores indexed by [victim_piece] [attacker_piece]
const MVV_LVA_TABLE: [[i32; 12]; 13] = {
    let mut table = [[0; 12]; 13];

    let mut victim = 0;
    while victim < 12 {
        let mut attacker = 0;
        while attacker < 12{
            let victim_value = get_piece_score(victim);
            let attacker_value = get_piece_score(attacker);
            table[victim][attacker] = (victim_value * 10) - attacker_value + 100_000;
            attacker += 1;
        }
        victim += 1;
    }
    table
};

#[inline(always)]
/// Returns the value of the specified piece in centipawns.
pub const fn get_piece_score(piece_type: usize) -> i32 {
    match piece_type {
        piece::W_PAWN | piece::B_PAWN => PIECE_VALUES[0],
        piece::W_KNIGHT | piece::B_KNIGHT => PIECE_VALUES[1],
        piece::W_BISHOP | piece::B_BISHOP => PIECE_VALUES[2],
        piece::W_ROOK | piece::B_ROOK => PIECE_VALUES[3],
        piece::W_QUEEN | piece::B_QUEEN => PIECE_VALUES[4],
        piece::W_KING | piece::B_KING => PIECE_VALUES[5],
        _ => unreachable!(),
    }
}

/// Scores a move for move ordering using the MVV-LVA heuristic and promotion bonuses.
#[inline(always)]
pub fn mvv_lva(mv: Move) -> i32 {
    let victim = mv.captured().unwrap_or(12);
    let attacker = mv.piece_type();
    let move_type = mv.move_type() as usize;
    MVV_LVA_TABLE[victim][attacker] + PROMOTION_BONUSES[move_type]
}
