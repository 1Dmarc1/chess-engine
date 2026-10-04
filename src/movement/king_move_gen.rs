use crate::board::game_state::GameState;
use crate::movement::{Move, MoveList};
use crate::types::piece::PieceColor;
use crate::types::{
    CASTLE_BLACK_KING, CASTLE_BLACK_QUEEN, CASTLE_WHITE_KING, CASTLE_WHITE_QUEEN, MoveType, piece,
};

/// Generates castle moves depending on the game state and appends them to the move list.
pub fn generate_castle_moves(from_square: u8, state: &GameState, move_list: &mut MoveList) {
    if state.is_in_check(state.board.side_to_move) {
        return; // Can not castle while in check
    }

    let enemy_side = !state.board.side_to_move;
    let all_pieces = state.all_pieces();

    if state.board.side_to_move == PieceColor::White {
        // White Kingside
        if (state.castling_rights & CASTLE_WHITE_KING) != 0 {
            const F1: u8 = 5;
            const G1: u8 = 6;

            let path_clear = !all_pieces.is_set(F1) && !all_pieces.is_set(G1);
            let path_safe = !state.is_square_attacked(F1, enemy_side)
                && !state.is_square_attacked(G1, enemy_side);

            if path_clear && path_safe {
                move_list.push(Move::new(
                    from_square,
                    G1,
                    piece::W_KING,
                    None,
                    MoveType::KingCastle,
                ));
            }
        }

        // White Queenside
        if (state.castling_rights & CASTLE_WHITE_QUEEN) != 0 {
            const B1: u8 = 1;
            const C1: u8 = 2;
            const D1: u8 = 3;

            // B1, C1, D1 must all be empty
            let path_clear =
                !all_pieces.is_set(B1) && !all_pieces.is_set(C1) && !all_pieces.is_set(D1);

            // King passes through D1 and C1 (B1 can be attacked)
            let path_safe = !state.is_square_attacked(D1, enemy_side)
                && !state.is_square_attacked(C1, enemy_side);

            if path_clear && path_safe {
                move_list.push(Move::new(
                    from_square,
                    C1,
                    piece::W_KING,
                    None,
                    MoveType::QueenCastle,
                ));
            }
        }
    } else {
        // Black Kingside
        if (state.castling_rights & CASTLE_BLACK_KING) != 0 {
            const F8: u8 = 61;
            const G8: u8 = 62;

            let path_clear = !all_pieces.is_set(F8) && !all_pieces.is_set(G8);
            let path_safe = !state.is_square_attacked(F8, enemy_side)
                && !state.is_square_attacked(G8, enemy_side);

            if path_clear && path_safe {
                move_list.push(Move::new(
                    from_square,
                    G8,
                    piece::B_KING,
                    None,
                    MoveType::KingCastle,
                ));
            }
        }

        // Black Queenside
        if (state.castling_rights & CASTLE_BLACK_QUEEN) != 0 {
            const B8: u8 = 57;
            const C8: u8 = 58;
            const D8: u8 = 59;

            let path_clear =
                !all_pieces.is_set(B8) && !all_pieces.is_set(C8) && !all_pieces.is_set(D8);

            let path_safe = !state.is_square_attacked(D8, enemy_side)
                && !state.is_square_attacked(C8, enemy_side);

            if path_clear && path_safe {
                move_list.push(Move::new(
                    from_square,
                    C8,
                    piece::B_KING,
                    None,
                    MoveType::QueenCastle,
                ));
            }
        }
    }
}
