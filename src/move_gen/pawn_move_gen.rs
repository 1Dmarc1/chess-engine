use crate::board::bitboard::Bitboard;
use crate::board::game_state::GameState;
use crate::move_gen::{MoveList, MoveType, attacks};
use crate::types::{Move, piece};
use crate::types::piece::PieceColor;

pub fn generate_pawn_moves<const CAPTURES: bool, const QUIETS: bool>(
    from_square: u8,
    state: &GameState,
    move_list: &mut MoveList,
) {
    let is_about_to_promote = if state.board.side_to_move == PieceColor::White {
        from_square >= 48 && from_square <= 55 // White rank 7
    } else {
        from_square >= 8 && from_square <= 15 // Black rank 2
    };

    if QUIETS || (CAPTURES && is_about_to_promote) {
        if generate_single_push(from_square, state, move_list) {
            if QUIETS && !is_about_to_promote {
                generate_double_push(from_square, state, move_list); // Only generate the double push if the single push is possible
            }
        }
    }

    if CAPTURES {
        generate_attacks(from_square, state, move_list);
    }
}

/// Generates all single pushes and append them to the move list.
/// Returns true if a move was generated, else false.
fn generate_single_push(from_square: u8, state: &GameState, move_list: &mut MoveList) -> bool {
    // Pawns on the very top or bottom can't push
    if state.board.side_to_move == PieceColor::White {
        if from_square >= 56 {
            return false;
        }
    } else {
        if from_square < 8 {
            return false;
        }
    }

    let (piece, to) = if state.board.side_to_move == PieceColor::White {
        (piece::W_PAWN, from_square + 8)
    } else {
        (piece::B_PAWN, from_square - 8)
    };

    let move_bb = Bitboard(1u64 << to);

    // Check if the target square is blocked
    if (move_bb & state.all_pieces()).0 != 0 {
        false // Blocked: No single push possible
    } else {
        // Check for promotion
        if is_promo_square(to, state.board.side_to_move) {
            push_promotion_moves(from_square, to, None, state.board.side_to_move, move_list);
        } else {
            move_list.push(Move::new(from_square, to, piece, None, MoveType::Quiet));
        }
        true
    }
}

/// Generate all double pushes and append them to the move list.
fn generate_double_push(from_square: u8, state: &GameState, move_list: &mut MoveList) {
    let (moving_pawn, to) = if state.board.side_to_move == PieceColor::White {
        if !(8..=15).contains(&from_square) {
            return;
        } // Rank 2
        (piece::W_PAWN, from_square + 16)
    } else {
        if !(48..=55).contains(&from_square) {
            return;
        } // Rank 7
        (piece::B_PAWN, from_square - 16)
    };

    let push_bb = 1u64 << to;

    // Target square must be empty
    if (state.all_pieces().0 & push_bb) != 0 {
        return;
    }

    move_list.push(Move::new(
        from_square,
        to,
        moving_pawn,
        None,
        MoveType::DoublePawnPush,
    ));
}

/// Generate the pawns attacks
fn generate_attacks(from_square: u8, state: &GameState, move_list: &mut MoveList) {
    let moving_pawn = if state.board.side_to_move == PieceColor::White {
        piece::W_PAWN
    } else {
        piece::B_PAWN
    };
    let enemy_pawn = if state.board.side_to_move == PieceColor::White {
        piece::B_PAWN
    } else {
        piece::W_PAWN
    };

    let pawn_attacks_bb = if state.board.side_to_move == PieceColor::White {
        attacks::WHITE_PAWN_ATTACKS[from_square as usize]
    } else {
        attacks::BLACK_PAWN_ATTACKS[from_square as usize]
    };

    // Standard Captures
    let mut valid_captures = Bitboard(pawn_attacks_bb.0 & state.opponent_pieces().0);

    while !valid_captures.is_empty(){
        let target_square = valid_captures.pop_lsb();
        let captured_piece = state.board.get_piece_at_square(target_square);

        if is_promo_square(target_square, state.board.side_to_move) {
            push_promotion_moves(
                from_square,
                target_square,
                captured_piece,
                state.board.side_to_move,
                move_list,
            );
        } else {
            move_list.push(Move::new(
                from_square,
                target_square,
                moving_pawn,
                captured_piece,
                MoveType::Capture,
            ));
        }
    }

    // En Passant Captures
    if let Some(ep_sq) = state.en_passant {
        let ep_bb = 1u64 << ep_sq;
        if (pawn_attacks_bb.0 & ep_bb) != 0 {
            move_list.push(Move::new(
                from_square,
                ep_sq as u8,
                moving_pawn,
                Some(enemy_pawn),
                MoveType::EnPassant,
            ));
        }
    }
}

fn push_promotion_moves(
    from: u8,
    to: u8,
    captured: Option<usize>,
    piece_color: PieceColor,
    move_list: &mut MoveList,
) {
    let is_capture = captured.is_some();
    let piece;

    let promotions = if piece_color == PieceColor::White {
        piece = piece::W_PAWN;
        if is_capture {
            [
                (piece::W_QUEEN, MoveType::QueenPromoCapture),
                (piece::W_ROOK, MoveType::RookPromoCapture),
                (piece::W_BISHOP, MoveType::BishopPromoCapture),
                (piece::W_KNIGHT, MoveType::KnightPromoCapture),
            ]
        } else {
            [
                (piece::W_QUEEN, MoveType::QueenPromotion),
                (piece::W_ROOK, MoveType::RookPromotion),
                (piece::W_BISHOP, MoveType::BishopPromotion),
                (piece::W_KNIGHT, MoveType::KnightPromotion),
            ]
        }
    } else {
        piece = piece::B_PAWN;
        if is_capture {
            [
                (piece::B_QUEEN, MoveType::QueenPromoCapture),
                (piece::B_ROOK, MoveType::RookPromoCapture),
                (piece::B_BISHOP, MoveType::BishopPromoCapture),
                (piece::B_KNIGHT, MoveType::KnightPromoCapture),
            ]
        } else {
            [
                (piece::B_QUEEN, MoveType::QueenPromotion),
                (piece::B_ROOK, MoveType::RookPromotion),
                (piece::B_BISHOP, MoveType::BishopPromotion),
                (piece::B_KNIGHT, MoveType::KnightPromotion),
            ]
        }
    };

    for (_promoted_piece, move_type) in promotions {
        move_list.push(Move::new(from, to, piece, captured, move_type));
    }
}

/// Returns true if square is a promotion square
fn is_promo_square(square: u8, color: PieceColor) -> bool {
    if color == PieceColor::White {
        square >= 56 && square <= 63
    } else {
        square <= 7
    }
}
