use crate::types::{Move, MoveType, piece};

const PIECE_VALUES: [i32; 6] = [
    100,    // Pawn
    320,    // Knight
    330,    // Bishop
    500,    // Rook
    900,    // Queen
    20_000, // King
];

/// Returns the value of the specified piece in centipawns.
pub fn get_piece_score(piece_type: usize) -> i32 {
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
pub fn mvv_lva(mv: &Move) -> i32 {
    let mut final_score = 0;

    // Handle captures
    if let Some(captured_piece) = mv.captured() {
        let captured_value = get_piece_score(captured_piece);
        let attacker_value = get_piece_score(mv.piece_type());
        final_score = (captured_value * 10) - attacker_value + 100_000;
    }

    // Handle Promotions
    match mv.move_type() {
        MoveType::QueenPromotion | MoveType::QueenPromoCapture => {
            final_score += 900_000;
        }
        MoveType::RookPromotion | MoveType::RookPromoCapture => {
            final_score += 500_000;
        }
        MoveType::BishopPromotion | MoveType::BishopPromoCapture => {
            final_score += 300_000;
        }
        MoveType::KnightPromotion | MoveType::KnightPromoCapture => {
            final_score += 300_000;
        }
        _ => {}
    }

    final_score
}
