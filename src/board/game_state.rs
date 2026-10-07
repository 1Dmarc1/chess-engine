use crate::board::bitboard::Bitboard;
use crate::board::board_state::BoardState;
use crate::board::fen_parser::{ParseError, parse_fen};
use crate::board::history_stack::HistoryStack;
use crate::board::transposition_table::Zobrist;
use crate::search::evaluation;
use crate::search::search_worker::nnue_diff::NNUEDiff;
use crate::types::piece::PieceColor;
use crate::types::{CASTLING_MASKS, Move, MoveType, piece};


#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GameState {
    pub board: BoardState,
    pub castling_rights: u8,
    pub en_passant: Option<usize>,
    pub halfmove_clock: u16,
    pub fullmove_number: u32,

    pub hash: u64,
    pub ply: usize,
    pub stack : HistoryStack,

    /// 0 - White, 1 - Black, 2 - All
    pub occupancy: [Bitboard; 3],
}

impl GameState {
    pub fn new(
        board: BoardState,
        en_passant: Option<usize>,
        castling_rights: u8,
        halfmove_clock: u16,
        fullmove_number: u32,
    ) -> GameState {
        let mut state = GameState {
            board,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,

            hash: 0,
            ply: 0,
            stack: HistoryStack::new(),
            occupancy: [Bitboard(0); 3],
        };
        state.refresh_occupancy_boards();
        state
    }

    pub fn from_fen(fen: &str) -> Result<GameState, ParseError> {
        parse_fen(String::from(fen))
    }

    pub fn new_startpos() -> GameState {
        parse_fen(String::from(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ))
        .unwrap()
    }

    pub fn is_repetition(&self) -> bool {
        let max_lookback = (self.halfmove_clock as usize).min(self.ply); // Go as far back as the halfmove_clock
        for i in (2..=max_lookback).step_by(2) { // Step by 2 to skip the opponent
            if self.stack.get(self.ply - i).hash == self.hash {
                return true;
            }
        }
        false
    }

    pub fn make_move_if_legal(&mut self, mv: &Move, zobrist: &Zobrist) -> (bool, NNUEDiff) {
        let diff = self.make_move(mv, zobrist);

        if self.is_in_check(!self.board.side_to_move) {
            self.undo_move(mv);
            return (false, diff);
        }
        (true, diff)
    }

    pub fn make_null_move(&mut self, zobrist: &Zobrist) {
        self.push_to_history_stack();

        if self.board.side_to_move == PieceColor::Black {
            self.fullmove_number += 1; // Increment the fullmove number
        }

        // Toggle the side
        self.board.side_to_move = !self.board.side_to_move;
        self.hash ^= zobrist.side_key;

        if let Some(ep) = self.en_passant {
            self.hash ^= zobrist.en_passant_keys[ep % 8];
            self.en_passant = None;
        }
    }

    pub fn undo_null_move(&mut self) {
        self.board.side_to_move = !self.board.side_to_move;
        self.pop_from_history_stack(); // Restore state variables

        if self.board.side_to_move == PieceColor::Black {
            self.fullmove_number -= 1; // Decrement the fullmove number
        }
    }

    fn make_move(&mut self, mv: &Move, zobrist: &Zobrist) -> NNUEDiff {
        let mut diff = NNUEDiff::new(self.board);

        self.push_to_history_stack(); // Save current state to stack array

        if self.board.side_to_move == PieceColor::Black {
            self.fullmove_number += 1; // Increment the fullmove number
        }
        self.halfmove_clock += 1; // Increment the halfmove clock
        if mv.captured().is_some() || mv.piece_type() == piece::W_PAWN || mv.piece_type() == piece::B_PAWN {
            self.halfmove_clock = 0;
        }

        // Clear old en passant key from hash
        if let Some(ep_sq) = self.en_passant {
            self.hash ^= zobrist.en_passant_keys[ep_sq % 8];
            self.en_passant = None;
        }

        let from = mv.from();
        let to = mv.to();
        let piece_type = mv.piece_type();

        // Handle captures
        if let Some(captured_piece) = mv.captured(){
            let cap_sq = if mv.move_type() == MoveType::EnPassant {
                if self.board.side_to_move == PieceColor::White {
                    mv.to() - 8
                } else {
                    mv.to() + 8
                }
            } else {
                mv.to()
            };
            self.remove_piece(cap_sq, captured_piece, zobrist, &mut diff);
        }

        // Move the main piece
        if mv.is_promotion(){
            self.remove_piece(from, piece_type, zobrist, &mut diff);
            self.put_piece(to, mv.landed_piece(), zobrist, &mut diff);
        }
        else{
            self.move_piece(from, to, piece_type, zobrist, &mut diff);
        }

        // Handle Special moves
        match mv.move_type() {
            MoveType::DoublePawnPush => {
                let ep_sq = if self.board.side_to_move == PieceColor::White {
                    mv.to() - 8
                } else {
                    mv.to() + 8
                };
                self.en_passant = Some(ep_sq as usize);
                self.hash ^= zobrist.en_passant_keys[(ep_sq % 8) as usize];
            }
            MoveType::KingCastle => {
                let (r_type, r_from, r_to) = if self.board.side_to_move == PieceColor::White {
                    (piece::W_ROOK, 7, 5)
                } else {
                    (piece::B_ROOK, 63, 61)
                };
                self.move_piece(r_from, r_to, r_type, zobrist, &mut diff);
            }
            MoveType::QueenCastle => {
                let (r_type, r_from, r_to) = if self.board.side_to_move == PieceColor::White {
                    (piece::W_ROOK, 0, 3)
                } else {
                    (piece::B_ROOK, 56, 59)
                };
                self.move_piece(r_from, r_to, r_type, zobrist, &mut diff);
            }
            _ => {}
        }

        // Update Castling Rights
        self.hash ^= zobrist.castling_keys[self.castling_rights as usize]; // Remove previous
        self.castling_rights &=
            CASTLING_MASKS[mv.from() as usize] & CASTLING_MASKS[mv.to() as usize]; // Update with masks
        self.hash ^= zobrist.castling_keys[self.castling_rights as usize]; // Add new

        // Toggle Side
        self.board.side_to_move = !self.board.side_to_move;
        self.hash ^= zobrist.side_key;

        diff.target_state = self.board;
        diff
    }

    pub fn undo_move(&mut self, mv: &Move) {
        // Switch side back
        self.board.side_to_move = !self.board.side_to_move;

        if self.board.side_to_move == PieceColor::Black {
            self.fullmove_number -= 1; // Decrement the fullmove number
        }

        // Restore state variables
        self.pop_from_history_stack();

        // Move main piece back on bitboard
        self.board.pieces[mv.piece_type()].clear(mv.to());
        self.board.pieces[mv.piece_type()].set(mv.from());

        // Update occupancy boards
        let us = if self.board.side_to_move == PieceColor::White {
            0
        } else {
            1
        };
        let them = 1 - us;
        self.occupancy[us].set(mv.from());
        self.occupancy[us].clear(mv.to());

        // Put captured piece back (if any)
        if mv.move_type() == MoveType::EnPassant {
            let ep_pawn_sq = if self.board.side_to_move == PieceColor::White {
                mv.to() - 8
            } else {
                mv.to() + 8
            };
            let enemy_pawn = if self.board.side_to_move == PieceColor::White {
                piece::B_PAWN
            } else {
                piece::W_PAWN
            };
            self.board.pieces[enemy_pawn].set(ep_pawn_sq);
            self.occupancy[them].set(ep_pawn_sq);
        } else if let Some(captured_idx) = mv.captured() {
            self.board.pieces[captured_idx].set(mv.to());
            self.occupancy[them].set(mv.to());
        }

        // Handle promotion
        if mv.is_promotion() {
            let promo_piece = mv.landed_piece();
            self.board.pieces[promo_piece].clear(mv.to()); // Remove promoted piece
        }

        // Handle castling
        match mv.move_type() {
            MoveType::KingCastle => {
                if self.board.side_to_move == PieceColor::White {
                    self.board.pieces[piece::W_ROOK].clear(5);
                    self.board.pieces[piece::W_ROOK].set(7);

                    self.occupancy[us].clear(5);
                    self.occupancy[us].set(7);
                } else {
                    self.board.pieces[piece::B_ROOK].clear(61);
                    self.board.pieces[piece::B_ROOK].set(63);

                    self.occupancy[us].clear(61);
                    self.occupancy[us].set(63);
                }
            }
            MoveType::QueenCastle => {
                if self.board.side_to_move == PieceColor::White {
                    self.board.pieces[piece::W_ROOK].clear(3);
                    self.board.pieces[piece::W_ROOK].set(0);

                    self.occupancy[us].clear(3);
                    self.occupancy[us].set(0);
                } else {
                    self.board.pieces[piece::B_ROOK].clear(59);
                    self.board.pieces[piece::B_ROOK].set(56);

                    self.occupancy[us].clear(59);
                    self.occupancy[us].set(56);
                }
            }
            _ => {}
        }

        // Update the occupancy board
        self.occupancy[2] = self.occupancy[0] | self.occupancy[1];
    }

    #[inline(always)]
    pub fn white_pieces(&self) -> Bitboard {
        self.occupancy[0]
    }

    #[inline(always)]
    pub fn black_pieces(&self) -> Bitboard {
        self.occupancy[1]
    }

    #[inline(always)]
    pub fn all_pieces(&self) -> Bitboard {
        self.occupancy[2]
    }

    #[inline(always)]
    pub fn friendly_pieces(&self) -> Bitboard {
        if self.board.side_to_move == PieceColor::White {
            self.white_pieces()
        } else {
            self.black_pieces()
        }
    }

    #[inline(always)]
    pub fn opponent_pieces(&self) -> Bitboard {
        if self.board.side_to_move == PieceColor::White {
            self.black_pieces()
        } else {
            self.white_pieces()
        }
    }

    pub fn is_in_check(&self, color: PieceColor) -> bool {
        let mut king: Bitboard = if color == PieceColor::White {
            self.board.pieces[piece::W_KING]
        } else {
            self.board.pieces[piece::B_KING]
        };
        if !(king.0 == 0) {
            let square = king.pop_lsb();
            return self.is_square_attacked(square, !color);
        }
        false
    }

    /// Returns true if a square is attacked by the specified color.
    pub fn is_square_attacked(&self, sq: u8, attacker_color: PieceColor) -> bool {
        let occupied = self.all_pieces();
        let attacker_occ = if attacker_color == PieceColor::White {
            self.occupancy[0]
        } else {
            self.occupancy[1]
        };

        let attackers = self.board.all_attackers_to::<true>(sq, occupied, attacker_occ);
        !(attackers & attacker_occ).is_empty()
    }

    #[inline]
    /// Returns true iff the move's material gain is bigger or equal to threshold.
    pub fn is_move_greater_equal(&self, mv: Move, threshold: i32) -> bool {

        // What can we gain from that move
        let cap_val = match mv.captured() {
            Some(piece) => evaluation::get_piece_score(piece),
            None => 0,
        };

        let attacker_val = evaluation::get_piece_score(mv.piece_type());
        if cap_val - attacker_val >= threshold {
            return true;
        }

        let mut swap = cap_val - threshold;
        if swap < 0 {
            return false; // Initial capture doesn't even reach threshold
        }

        // What do we risk losing next
        let next_victim = mv.landed_piece();
        swap -= evaluation::get_piece_score(next_victim);
        if swap >= 0 {
            return true; // Even if we lose our piece, we still meet/exceed threshold
        }

        // Remove the initial attacker
        let mut occupied = self.occupancy[2];
        occupied.clear(mv.from());

        let mut side = !self.board.side_to_move;
        let mut res = side;

        loop {
            let attackers = self.board.all_attackers_to::<true>(mv.to(), occupied, self.occupancy[side as usize]) & occupied;
            if attackers.is_empty() {
                break;
            }

            let (attacker_sq, piece_type) = match self.pop_least_valuable(&attackers, side) {
                Some(r) => r,
                None => break,
            };

            occupied.clear(attacker_sq);
            let next_victim = piece_type;

            swap = -swap - 1 - evaluation::get_piece_score(next_victim);
            res = !res;

            if swap >= 0 {
                if (piece_type == piece::W_KING || piece_type == piece::B_KING)
                    && !(attackers & self.occupancy[(!side) as usize]).is_empty()
                {
                    res = !res; // Revert if king capture is illegal
                }
                break;
            }

            side = !side;
        }
        res != self.board.side_to_move
    }

    fn pop_least_valuable(&self, attackers_bb: &Bitboard, side: PieceColor) -> Option<(u8, usize)> {
        let pieces = if side == PieceColor::White {
            piece::WHITE_PIECES
        } else {
            piece::BLACK_PIECES
        };

        for &piece in &pieces {
            let mut subset = *attackers_bb & self.board.pieces[piece];

            if !subset.is_empty() {
                let lsb_sq = subset.pop_lsb();
                return Some((lsb_sq, piece));
            }
        }
        None
    }

    fn refresh_occupancy_boards(&mut self) {
        self.occupancy[0].0 = 0;

        self.occupancy[0] |= self.board.pieces[piece::W_PAWN];
        self.occupancy[0] |= self.board.pieces[piece::W_KNIGHT];
        self.occupancy[0] |= self.board.pieces[piece::W_BISHOP];
        self.occupancy[0] |= self.board.pieces[piece::W_ROOK];
        self.occupancy[0] |= self.board.pieces[piece::W_QUEEN];
        self.occupancy[0] |= self.board.pieces[piece::W_KING];

        self.occupancy[1].0 = 0;
        self.occupancy[1] |= self.board.pieces[piece::B_PAWN];
        self.occupancy[1] |= self.board.pieces[piece::B_KNIGHT];
        self.occupancy[1] |= self.board.pieces[piece::B_BISHOP];
        self.occupancy[1] |= self.board.pieces[piece::B_ROOK];
        self.occupancy[1] |= self.board.pieces[piece::B_QUEEN];
        self.occupancy[1] |= self.board.pieces[piece::B_KING];

        self.occupancy[2] = self.occupancy[0] | self.occupancy[1];
    }

    #[inline]
    fn push_to_history_stack(&mut self) {
        self.stack.push(self.hash, self.halfmove_clock, self.castling_rights, self.en_passant);
        self.ply += 1;
    }

    #[inline]
    fn pop_from_history_stack(&mut self) {
        self.ply -= 1;
        let entry = self.stack.pop();
        self.hash = entry.hash;
        self.castling_rights = entry.castling_rights;
        self.halfmove_clock = entry.half_move_clock;
        self.en_passant = entry.en_passant_sq();

    }
}


// --- Internal helpers
impl GameState{
    #[inline(always)]
    fn put_piece(&mut self, sq : u8, piece: usize, zobrist: &Zobrist, diff : &mut NNUEDiff) {
        let color_idx = (piece >= 6) as usize;

        self.board.pieces[piece].set(sq);

        self.occupancy[color_idx].set(sq);
        self.occupancy[2].set(sq);

        self.hash ^= zobrist.piece_keys[piece][sq as usize];

        diff.push_added(sq, piece);
    }

    #[inline(always)]
    fn remove_piece(&mut self, sq : u8, piece : usize, zobrist: &Zobrist, diff : &mut NNUEDiff) {
        let color_idx = (piece >= 6) as usize;

        self.board.pieces[piece].clear(sq);
        self.occupancy[color_idx].clear(sq);
        self.occupancy[2].clear(sq);

        self.hash ^= zobrist.piece_keys[piece][sq as usize];

        diff.push_removed(sq, piece);
    }

    #[inline(always)]
    fn move_piece(&mut self, from: u8, to :u8, piece : usize, zobrist: &Zobrist, diff : &mut NNUEDiff) {
        self.remove_piece(from, piece, zobrist, diff);
        self.put_piece(to, piece, zobrist, diff);
    }

}

impl Default for GameState {
    fn default() -> Self {
        let mut state = GameState {
            board: BoardState::new(),
            castling_rights: 0b00001111,
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 0,
            hash: 0,
            ply: 0,
            stack: HistoryStack::new(),
            occupancy: [Bitboard(0); 3],
        };
        state.refresh_occupancy_boards();
        state
    }
}
