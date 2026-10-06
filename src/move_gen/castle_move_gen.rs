use crate::board::bitboard::Bitboard;
use crate::board::game_state::GameState;
use crate::move_gen::{Move, MoveList};
use crate::types::piece::PieceColor;
use crate::types::{CASTLE_BLACK_KING, CASTLE_BLACK_QUEEN, CASTLE_WHITE_KING, CASTLE_WHITE_QUEEN};

const WHITE_KINGSIDE_PATH: Bitboard = Bitboard(1 << F1 | 1 << G1);
const F1: u8 = 5;
const G1: u8 = 6;
const WHITE_KINGSIDE_CASTLE_MOVE: Move = Move::from_raw(5198212);

const WHITE_QUEENSIDE_PATH: Bitboard = Bitboard(1 << B1 | 1 << C1 | 1 << D1);
const B1: u8 = 1;
const C1: u8 = 2;
const D1: u8 = 3;
const WHITE_QUEENSIDE_CASTLE_MOVE: Move = Move::from_raw(6246532);

const BLACK_KINGSIDE_PATH: Bitboard = Bitboard(1 << F8 | 1 << G8);
const F8: u8 = 61;
const G8: u8 = 62;
const BLACK_KINGSIDE_CASTLE_MOVE: Move = Move::from_raw(5226428);

const BLACK_QUEENSIDE_PATH: Bitboard = Bitboard(1 << B8 | 1 << C8 | 1 << D8);
const B8: u8 = 57;
const C8: u8 = 58;
const D8: u8 = 59;
const BLACK_QUEENSIDE_CASTLE_MOVE: Move = Move::from_raw(6274748);

fn generate_white_castle_moves(state: &GameState, move_list: &mut MoveList) {
    let rights = state.castling_rights;
    if (rights & (CASTLE_WHITE_KING | CASTLE_WHITE_QUEEN)) == 0 {
        return;
    }

    let enemy = PieceColor::Black;
    let all_pieces = state.all_pieces();

    // White Kingside
    if (state.castling_rights & CASTLE_WHITE_KING) != 0 {
        if (all_pieces & WHITE_KINGSIDE_PATH).is_empty()
            && !state.is_square_attacked(F1, enemy)
            && !state.is_square_attacked(G1, enemy)
        {
            move_list.push(WHITE_KINGSIDE_CASTLE_MOVE);
        }
    }

    // White Queenside
    if (state.castling_rights & CASTLE_WHITE_QUEEN) != 0 {
        if (all_pieces & WHITE_QUEENSIDE_PATH).is_empty()
            && !state.is_square_attacked(D1, enemy)
            && !state.is_square_attacked(C1, enemy)
        {
            move_list.push(WHITE_QUEENSIDE_CASTLE_MOVE);
        }
    }
}

fn generate_black_castle_moves(state: &GameState, move_list: &mut MoveList) {
    let enemy_side = !state.board.side_to_move;
    let all_pieces = state.all_pieces();

    // Black Kingside
    if (state.castling_rights & CASTLE_BLACK_KING) != 0 {
        if (all_pieces & BLACK_KINGSIDE_PATH).is_empty() && !state.is_square_attacked(F8, enemy_side)
            && !state.is_square_attacked(G8, enemy_side){
            move_list.push(BLACK_KINGSIDE_CASTLE_MOVE);
        }
    }

    // Black Queenside
    if (state.castling_rights & CASTLE_BLACK_QUEEN) != 0 {
        if (all_pieces & BLACK_QUEENSIDE_PATH).is_empty() && !state.is_square_attacked(D8, enemy_side)
            && !state.is_square_attacked(C8, enemy_side){
            move_list.push(BLACK_QUEENSIDE_CASTLE_MOVE);
        }
    }
}

/// Generates castle moves depending on the game state and appends them to the move list.
pub fn generate_castle_moves(state: &GameState, move_list: &mut MoveList) {
    if state.is_in_check(state.board.side_to_move) {
        return; // Can not castle while in check
    }

    if state.board.side_to_move == PieceColor::White {
        generate_white_castle_moves(state, move_list);
    } else {
        generate_black_castle_moves(state, move_list);
    }
}
