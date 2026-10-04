pub mod attacks;
pub mod bishop_move_gen;
pub mod king_move_gen;
mod pawn_move_gen;
pub mod rook_move_gen;

use crate::board::bitboard::Bitboard;
use crate::board::game_state::GameState;
use crate::movement::attacks::KING_ATTACKS;
use crate::movement::bishop_move_gen::init_bishop_tables;
use crate::movement::king_move_gen::generate_castle_moves;
use crate::movement::rook_move_gen::init_rook_tables;
use crate::types::MoveType::Quiet;
use crate::types::piece::PieceColor;
use crate::types::{Move, MoveType, piece};

#[derive(Copy, Clone)]
pub struct MoveList {
    pub moves: [Move; 128],
    pub len: usize,
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, mov: Move) {
        self.moves[self.len] = mov;
        self.len += 1;
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            moves: [Move::new(0, 0, 0, None, Quiet); 128],
            len: 0,
        }
    }
}

/// Initializes all tables required for move generation.
pub fn setup() {
    init_bishop_tables();
    init_rook_tables();
}

/// Generates all possible pseudo legal moves which are possible in the current state.
pub fn generate_pseudo_legal_moves<const CAPTURES: bool, const QUIETS: bool>(
    state: &GameState,
    move_list: &mut MoveList,
) {
    let target_mask = if CAPTURES && QUIETS {
        !state.friendly_pieces() // All squares with no friendly pieces
    } else if CAPTURES {
        state.opponent_pieces() // Only squares with opponent pieces
    } else if QUIETS {
        !state.all_pieces() // Only empty squares
    } else {
        return;
    };

    // Generate Pawn moves
    let mut my_pawns: Bitboard = if state.board.side_to_move == PieceColor::White {
        state.board.pieces[piece::W_PAWN]
    } else {
        state.board.pieces[piece::B_PAWN]
    };
    while !my_pawns.is_empty() {
        let square = my_pawns.pop_lsb();
        pawn_move_gen::generate_pawn_moves::<CAPTURES, QUIETS>(square, state, move_list);
    }

    // Generate Knight moves
    let my_knight = if state.board.side_to_move == PieceColor::White {
        piece::W_KNIGHT
    } else {
        piece::B_KNIGHT
    };
    let mut my_knights: Bitboard = state.board.pieces[my_knight];
    while !my_knights.is_empty() {
        let square = my_knights.pop_lsb();
        generate_knight_moves(square, state, move_list, target_mask, my_knight);
    }

    // Generate Rook moves
    let my_rook = if state.board.side_to_move == PieceColor::White {
        piece::W_ROOK
    } else {
        piece::B_ROOK
    };
    let mut my_rooks: Bitboard = state.board.pieces[my_rook];
    while !my_rooks.is_empty() {
        let square = my_rooks.pop_lsb();
        generate_rook_moves(square, state, move_list, target_mask, my_rook)
    }

    // Generate Queen moves
    let my_queen = if state.board.side_to_move == PieceColor::White {
        piece::W_QUEEN
    } else {
        piece::B_QUEEN
    };
    let mut my_queens: Bitboard = state.board.pieces[my_queen];
    while !my_queens.is_empty() {
        let square = my_queens.pop_lsb();
        generate_queen_moves(square, state, move_list, target_mask, my_queen);
    }

    // Generate King moves
    let my_king = if state.board.side_to_move == PieceColor::White {
        piece::W_KING
    } else {
        piece::B_KING
    };
    let mut my_kings: Bitboard = state.board.pieces[my_king];
    while !my_kings.is_empty() {
        let square = my_kings.pop_lsb();
        generate_king_moves::<QUIETS>(square, state, move_list, target_mask, my_king);
    }

    // Generate Bishop moves
    let my_bishop = if state.board.side_to_move == PieceColor::White {
        piece::W_BISHOP
    } else {
        piece::B_BISHOP
    };
    let mut my_bishops: Bitboard = state.board.pieces[my_bishop];
    while !my_bishops.is_empty() {
        let square = my_bishops.pop_lsb();
        generate_bishop_moves(square, state, move_list, target_mask, my_bishop)
    }
}

#[inline]
fn generate_knight_moves(
    from_square: u8,
    state: &GameState,
    move_list: &mut MoveList,
    target_mask: Bitboard,
    piece_type: usize,
) {
    let attacks: Bitboard = attacks::KNIGHT_ATTACKS[from_square as usize] & target_mask;
    extract_moves(from_square, piece_type, move_list, attacks, state);
}

#[inline]
fn generate_rook_moves(
    from_square: u8,
    state: &GameState,
    move_list: &mut MoveList,
    target_mask: Bitboard,
    piece_type: usize,
) {
    let attacks: Bitboard =
        rook_move_gen::get_rook_attacks(from_square, state.all_pieces()) & target_mask;
    extract_moves(from_square, piece_type, move_list, attacks, state);
}

#[inline]
fn generate_bishop_moves(
    from_square: u8,
    state: &GameState,
    move_list: &mut MoveList,
    target_mask: Bitboard,
    piece_type: usize,
) {
    let attacks: Bitboard =
        bishop_move_gen::get_bishop_attacks(from_square, state.all_pieces()) & target_mask;
    extract_moves(from_square, piece_type, move_list, attacks, state);
}

#[inline]
fn generate_king_moves<const QUIETS: bool>(
    from_square: u8,
    state: &GameState,
    move_list: &mut MoveList,
    target_mask: Bitboard,
    piece_type: usize,
) {
    let attacks: Bitboard = KING_ATTACKS[from_square as usize] & target_mask;
    extract_moves(from_square, piece_type, move_list, attacks, state);

    if QUIETS {
        generate_castle_moves(state, move_list)
    }
}

#[inline]
fn generate_queen_moves(
    from_sq: u8,
    state: &GameState,
    mv_list: &mut MoveList,
    mask: Bitboard,
    piece: usize,
) {
    let mut attacks = rook_move_gen::get_rook_attacks(from_sq, state.all_pieces())
        | bishop_move_gen::get_bishop_attacks(from_sq, state.all_pieces());
    attacks = attacks & mask;
    extract_moves(from_sq, piece, mv_list, attacks, state);
}

#[inline]
fn extract_moves(
    from_square: u8,
    piece: usize,
    move_list: &mut MoveList,
    mut attacks: Bitboard,
    state: &GameState,
) {
    let enemies = state.opponent_pieces();

    while !attacks.is_empty() {
        let to = attacks.pop_lsb();

        let mut captured = None;
        let mut move_type = Quiet;
        if enemies.is_set(to) {
            captured = state.board.get_piece_at_square(to);
            move_type = MoveType::Capture
        }

        move_list.push(Move::new(from_square, to, piece, captured, move_type));
    }
}
