use nnue_rs::{Color, Piece, PieceKind};
use crate::board::board_state::BoardState;

#[derive(Copy, Clone)]
pub struct NNUEDiff {
    pub start_state: BoardState,
    pub target_state : BoardState,
    pub removed: [(u8, Piece); 2],
    pub removed_len: usize,
    pub added: [(u8, Piece); 2],
    pub added_len: usize,
}

impl NNUEDiff {
    #[inline(always)]
    pub fn new(state : BoardState) -> NNUEDiff {
        NNUEDiff {
            start_state: state,
            target_state: state,
            removed: [(0, Piece::new(Color::White, PieceKind::Pawn)); 2],
            removed_len: 0,
            added: [(0, Piece::new(Color::White, PieceKind::Pawn)); 2],
            added_len: 0,
        }
    }

    #[inline(always)]
    pub fn get_removed(&self) -> &[(u8, Piece)] {
        &self.removed[..self.removed_len]
    }

    #[inline(always)]
    pub fn get_added(&self) -> &[(u8, Piece)] {
        &self.added[..self.added_len]
    }

    #[inline(always)]
    pub fn push_removed(&mut self, sq: u8, piece: usize) {
        self.removed[self.removed_len] = (sq, Self::piece_to_nnue_piece(piece));
        self.removed_len += 1;
    }

    #[inline(always)]
    pub fn push_added(&mut self, sq: u8, piece: usize) {
        self.added[self.added_len] = (sq, Self::piece_to_nnue_piece(piece));
        self.added_len += 1;
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.removed_len = 0;
        self.added_len = 0;
    }

    #[inline]
    pub fn piece_to_nnue_piece(piece: usize) -> Piece {
        const MAP: [Piece; 12] = [
            Piece {
                kind: PieceKind::Pawn,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::Knight,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::Bishop,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::Rook,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::Queen,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::King,
                color: Color::White,
            },
            Piece {
                kind: PieceKind::Pawn,
                color: Color::Black,
            },
            Piece {
                kind: PieceKind::Knight,
                color: Color::Black,
            },
            Piece {
                kind: PieceKind::Bishop,
                color: Color::Black,
            },
            Piece {
                kind: PieceKind::Rook,
                color: Color::Black,
            },
            Piece {
                kind: PieceKind::Queen,
                color: Color::Black,
            },
            Piece {
                kind: PieceKind::King,
                color: Color::Black,
            },
        ];
        MAP[piece]
    }
}
