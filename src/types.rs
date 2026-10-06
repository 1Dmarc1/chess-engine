use crate::board::bitboard::Bitboard;
use crate::board::game_state::GameState;
use crate::move_gen::{attacks, bishop_move_gen, rook_move_gen};
use crate::types::MoveType::Quiet;
use crate::types::piece::PieceColor;
use crate::util;

pub const CASTLE_BLACK_QUEEN: u8 = 0b0001;
pub const CASTLE_BLACK_KING: u8 = 0b0010;
pub const CASTLE_WHITE_KING: u8 = 0b0100;
pub const CASTLE_WHITE_QUEEN: u8 = 0b1000;

pub mod piece {
    use std::ops::Not;

    pub const W_PAWN: usize = 0;
    pub const W_KNIGHT: usize = 1;
    pub const W_BISHOP: usize = 2;
    pub const W_ROOK: usize = 3;
    pub const W_QUEEN: usize = 4;
    pub const W_KING: usize = 5;
    pub const B_PAWN: usize = 6;
    pub const B_KNIGHT: usize = 7;
    pub const B_BISHOP: usize = 8;
    pub const B_ROOK: usize = 9;
    pub const B_QUEEN: usize = 10;
    pub const B_KING: usize = 11;

    pub const WHITE_PIECES: [usize; 6] = [W_PAWN, W_KNIGHT, W_BISHOP, W_ROOK, W_QUEEN, W_KING];

    pub const BLACK_PIECES: [usize; 6] = [B_PAWN, B_KNIGHT, B_BISHOP, B_ROOK, B_QUEEN, B_KING];

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum PieceColor {
        White,
        Black,
    }

    impl Not for PieceColor {
        type Output = PieceColor;
        fn not(self) -> Self::Output {
            if self == PieceColor::Black {
                PieceColor::White
            } else {
                PieceColor::Black
            }
        }
    }
}

/// A struct representing a basic move.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Move(u32);

impl Move {
    const NO_CAPTURE: u8 = 15;
    #[inline]
    pub fn new(
        from: u8,
        to: u8,
        piece_type: usize,
        captured: Option<usize>,
        move_type: MoveType,
    ) -> Move {
        let mut data = 0u32;
        data |= (from as u32) & 0x3F; // Bits 0-5 (6)
        data |= ((to as u32) & 0x3F) << 6; // Bits 6 - 11 (6)
        data |= (piece_type as u32 & 0xF) << 12; // Bits 12-15 (4)
        data |= (captured.unwrap_or(Self::NO_CAPTURE as usize) as u32 & 0xF) << 16; // Bits 16-19 (4)
        data |= (move_type as u8 as u32 & 0xF) << 20; // Bits 20-23 (4)
        Move(data)
    }

    #[inline(always)]
    pub const fn from_raw(data: u32) -> Move {
        Move(data)
    }

    pub fn to_uci(&self) -> Option<String> {
        let from_square = util::square_to_algebraic_notation(self.from())?;
        let to_square = util::square_to_algebraic_notation(self.to())?;

        let promo = match self.move_type() {
            MoveType::QueenPromotion | MoveType::QueenPromoCapture => "q",
            MoveType::RookPromotion | MoveType::RookPromoCapture => "r",
            MoveType::BishopPromotion | MoveType::BishopPromoCapture => "b",
            MoveType::KnightPromotion | MoveType::KnightPromoCapture => "n",
            _ => "",
        };
        Some(format!("{}{}{}", from_square, to_square, promo))
    }

    #[inline(always)]
    pub const fn get_raw(&self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub const fn from(&self) -> u8 {
        (self.0 & 0x3F) as u8
    }

    #[inline(always)]
    pub const fn to(&self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }

    #[inline(always)]
    pub const fn piece_type(&self) -> usize {
        ((self.0 >> 12) & 0xF) as usize
    }

    #[inline]
    pub const fn captured(&self) -> Option<usize> {
        let cap = ((self.0 >> 16) & 0xF) as u8;
        if cap == Self::NO_CAPTURE {
            None
        } else {
            Some(cap as usize)
        }
    }

    #[inline(always)]
    pub const fn move_type(&self) -> MoveType {
        let val = ((self.0 >> 20) & 0xF) as u8;
        MoveType::from_u8(val)
    }

    #[inline]
    pub const fn is_promotion(&self) -> bool {
        matches!(
            self.move_type(),
            MoveType::QueenPromotion
                | MoveType::QueenPromoCapture
                | MoveType::RookPromotion
                | MoveType::RookPromoCapture
                | MoveType::BishopPromotion
                | MoveType::BishopPromoCapture
                | MoveType::KnightPromotion
                | MoveType::KnightPromoCapture
        )
    }

    #[inline(always)]
    pub const fn is_en_passant(&self) -> bool {
        matches!(self.move_type(), MoveType::EnPassant)
    }

    #[inline]
    pub const fn is_castle(&self) -> bool {
        matches!(
            self.move_type(),
            MoveType::KingCastle | MoveType::QueenCastle
        )
    }

    #[inline]
    pub const fn landed_piece(&self) -> usize {
        let is_white = self.piece_type() < 6;
        match self.move_type() {
            MoveType::QueenPromotion | MoveType::QueenPromoCapture => {
                if is_white {
                    piece::W_QUEEN
                } else {
                    piece::B_QUEEN
                }
            }
            MoveType::RookPromotion | MoveType::RookPromoCapture => {
                if is_white {
                    piece::W_ROOK
                } else {
                    piece::B_ROOK
                }
            }
            MoveType::BishopPromotion | MoveType::BishopPromoCapture => {
                if is_white {
                    piece::W_BISHOP
                } else {
                    piece::B_BISHOP
                }
            }
            MoveType::KnightPromotion | MoveType::KnightPromoCapture => {
                if is_white {
                    piece::W_KNIGHT
                } else {
                    piece::B_KNIGHT
                }
            }
            _ => self.piece_type(),
        }
    }

    /// Returns true if this move is pseudo legal.
    #[inline]
    pub fn is_pseudo_legal(&self, state: &GameState) -> bool {

        let to = self.to();
        let from = self.from();
        let move_type = self.move_type();

        let to_bb = Bitboard(1u64 << to);
        let from_bb = Bitboard(1u64 << from);


        if (state.friendly_pieces() & from_bb).is_empty() {
            return false;
        }

        // Does the piece even exist
        if !state.board.pieces[self.piece_type()].is_set(from) {
            return false;
        }

        // We cant capture our own piece
        if !(state.friendly_pieces() & to_bb).is_empty() {
            return false;
        }

        let is_enemy_on_to = !(state.opponent_pieces() & to_bb).is_empty();
        if is_enemy_on_to != self.captured().is_some() && !self.is_en_passant() {
            return false;
        }

        match self.piece_type() {
            piece::W_KNIGHT | piece::B_KNIGHT => {
                !(attacks::KNIGHT_ATTACKS[from as usize] & to_bb).is_empty()
            },
            piece::W_KING | piece::B_KING => {
                if move_type == MoveType::KingCastle || move_type == MoveType::QueenCastle {
                    return false; // TODO: Implement Castle Checks
                }
                !(attacks::KING_ATTACKS[self.from() as usize] & to_bb).is_empty()
            }
            piece::W_BISHOP | piece::B_BISHOP => {
                let attacks = bishop_move_gen::get_bishop_attacks(from, state.all_pieces());
                !(attacks & to_bb).is_empty()
            }
            piece::W_ROOK | piece::B_ROOK => {
                let attacks = rook_move_gen::get_rook_attacks(from, state.all_pieces());
                !(attacks & to_bb).is_empty()
            }
            piece::W_QUEEN | piece::B_QUEEN => {
                let attacks = bishop_move_gen::get_bishop_attacks(from, state.all_pieces())
                    | rook_move_gen::get_rook_attacks(from, state.all_pieces());
                !(attacks & to_bb).is_empty()
            }
            piece::W_PAWN | piece::B_PAWN => {
                let us = state.board.side_to_move;
                let direction = if us == PieceColor::White { 1 } else { -1 };
                let diff = to as isize - from as isize;

                // Single push
                if diff == direction * 8 {
                    return state.board.get_piece_at_square(to).is_none();
                }

                // Double push
                if move_type == MoveType::DoublePawnPush {
                    let intermediate_sq = (from as isize + direction * 8) as u8;
                    return diff == direction * 16
                        && state.board.get_piece_at_square(to).is_none()
                        && state.board.get_piece_at_square(intermediate_sq).is_none();
                }

                // Diagonal captures & En Passant
                if diff == direction * 7 || diff == direction * 9 {
                    if move_type == MoveType::EnPassant {
                        return state.en_passant == Some(to as usize);
                    }
                    return state.board.get_piece_at_square(to).is_some();
                }

                false            }
            _ => unreachable!(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum MoveType {
    Quiet,
    DoublePawnPush,
    Capture,
    EnPassant,

    KingCastle,
    QueenCastle,

    QueenPromotion,
    RookPromotion,
    BishopPromotion,
    KnightPromotion,

    QueenPromoCapture,
    RookPromoCapture,
    BishopPromoCapture,
    KnightPromoCapture,
}

impl MoveType {
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        match value {
            0 => Quiet,
            1 => MoveType::DoublePawnPush,
            2 => MoveType::Capture,
            3 => MoveType::EnPassant,

            4 => MoveType::KingCastle,
            5 => MoveType::QueenCastle,

            6 => MoveType::QueenPromotion,
            7 => MoveType::RookPromotion,
            8 => MoveType::BishopPromotion,
            9 => MoveType::KnightPromotion,

            10 => MoveType::QueenPromoCapture,
            11 => MoveType::RookPromoCapture,
            12 => MoveType::BishopPromoCapture,
            13 => MoveType::KnightPromoCapture,
            _ => Quiet,
        }
    }
}

pub const CASTLING_MASKS: [u8; 64] = [
    // Rank 1 (a1=7, e1=3, h1=11)
    7, 15, 15, 15, 3, 15, 15, 11, // Rank 2
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 3
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 4
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 5
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 6
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 7
    15, 15, 15, 15, 15, 15, 15, 15, // Rank 8 (a8=14, e8=12, h8=13)
    14, 15, 15, 15, 12, 15, 15, 13,
];
