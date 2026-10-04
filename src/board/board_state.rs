use crate::board::bitboard::Bitboard;
use crate::types::piece;
use crate::types::piece::PieceColor;
use nnue_rs::{Board, Color, Piece, PieceKind};
use crate::movement::{attacks, bishop_move_gen, rook_move_gen};
use crate::movement::attacks::KING_ATTACKS;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoardState {
    /// Represents all pieces on the board. Uses on bitboard for each piece type and color.
    pub pieces: [Bitboard; 12],
    /// The color that is next to move.
    pub side_to_move: PieceColor,
}

impl BoardState {
    pub(crate) fn new() -> BoardState {
        BoardState {
            pieces: [Bitboard(0); 12],
            side_to_move: PieceColor::White,
        }
    }

    /// Returns a bitboard containing ALL pieces which could attack the specified square.
    /// If EARLY_EXIT is true, it returns immediately after finding the first attacker which is specified by the enemy_occupancy bitboard.
    #[inline]
    pub fn all_attackers_to<const EARLY_EXIT : bool>(&self, sq : u8, occupied : Bitboard, enemy_occupancy: Bitboard) -> Bitboard{
        let sq_idx = sq as usize;
        let mut attackers = Bitboard(0);

        // Pawn attacks
        let w_pawns = self.pieces[piece::W_PAWN];
        let b_pawns = self.pieces[piece::B_PAWN];
        attackers |= attacks::BLACK_PAWN_ATTACKS[sq_idx] & w_pawns;
        attackers |= attacks::WHITE_PAWN_ATTACKS[sq_idx] & b_pawns;
        if EARLY_EXIT && !(attackers & enemy_occupancy).is_empty() { return attackers; }

        // Knights
        let knights = self.pieces[piece::W_KNIGHT] | self.pieces[piece::B_KNIGHT];
        attackers |= attacks::KNIGHT_ATTACKS[sq_idx] & knights;
        if EARLY_EXIT && !(attackers & enemy_occupancy).is_empty() { return attackers; }

        // Kings
        let kings = self.pieces[piece::W_KING] | self.pieces[piece::B_KING];
        attackers |= KING_ATTACKS[sq_idx] & kings;
        if EARLY_EXIT && !(attackers & enemy_occupancy).is_empty() { return attackers; }

        // Rooks & Queens
        let rooks_queens = self.pieces[piece::W_ROOK] | self.pieces[piece::B_ROOK]
            | self.pieces[piece::W_QUEEN] | self.pieces[piece::B_QUEEN];
        if rooks_queens.0 != 0 {
            attackers |= rook_move_gen::get_rook_attacks(sq, occupied) & rooks_queens;
        }
        if EARLY_EXIT && !(attackers & enemy_occupancy).is_empty() { return attackers; }

        // Bishop & Queens
        let bishops_queens = self.pieces[piece::W_BISHOP] | self.pieces[piece::B_BISHOP]
            | self.pieces[piece::W_QUEEN] | self.pieces[piece::B_QUEEN];
        if bishops_queens.0 != 0 {
            attackers |= bishop_move_gen::get_bishop_attacks(sq, occupied) & bishops_queens;
        }

        attackers
    }

    #[inline(always)]
    pub fn get_piece_at_square(&self, square: u8) -> Option<usize> {
        for i in 0..12 {
            if self.pieces[i].is_set(square) {
                return Some(i);
            }
        }
        None
    }



    pub fn print_board(&self) {
        println!("  +-----------------+");
        // Loop from rank 7 down to 0 (Rank 8 down to Rank 1)
        for rank in (0..8).rev() {
            print!("{} | ", rank + 1);
            for file in 0..8 {
                let square = crate::util::rank_file_to_square(rank, file);
                let mut piece_char = '.';

                // Check all 12 piece bitboards to see which one occupies this square
                for i in 0..12 {
                    if self.pieces[i].is_set(square) {
                        piece_char = match i {
                            piece::W_PAWN => 'P',
                            piece::W_KNIGHT => 'N',
                            piece::W_BISHOP => 'B',
                            piece::W_ROOK => 'R',
                            piece::W_QUEEN => 'Q',
                            piece::W_KING => 'K',
                            piece::B_PAWN => 'p',
                            piece::B_KNIGHT => 'n',
                            piece::B_BISHOP => 'b',
                            piece::B_ROOK => 'r',
                            piece::B_QUEEN => 'q',
                            piece::B_KING => 'k',
                            _ => unreachable!(),
                        };
                        break; // Found the piece on this square, stop looking
                    }
                }
                print!("{} ", piece_char);
            }
            println!("|");
        }
        println!("  +-----------------+");
        println!("    a b c d e f g h\n");
    }
}

impl Board for BoardState {
    fn side_to_move(&self) -> Color {
        match self.side_to_move {
            PieceColor::White => Color::White,
            PieceColor::Black => Color::Black,
        }
    }

    fn king_square(&self, color: Color) -> u8 {
        let bb = match color {
            Color::White => self.pieces[piece::W_KING],
            Color::Black => self.pieces[piece::B_KING],
        };
        bb.get_lsb()
    }

    fn for_each_piece(&self, f: &mut dyn FnMut(u8, Piece)) {
        const PIECE_MAP: [(usize, PieceKind, Color); 12] = [
            (piece::W_PAWN, PieceKind::Pawn, Color::White),
            (piece::W_KNIGHT, PieceKind::Knight, Color::White),
            (piece::W_BISHOP, PieceKind::Bishop, Color::White),
            (piece::W_ROOK, PieceKind::Rook, Color::White),
            (piece::W_QUEEN, PieceKind::Queen, Color::White),
            (piece::W_KING, PieceKind::King, Color::White),
            (piece::B_PAWN, PieceKind::Pawn, Color::Black),
            (piece::B_KNIGHT, PieceKind::Knight, Color::Black),
            (piece::B_BISHOP, PieceKind::Bishop, Color::Black),
            (piece::B_ROOK, PieceKind::Rook, Color::Black),
            (piece::B_QUEEN, PieceKind::Queen, Color::Black),
            (piece::B_KING, PieceKind::King, Color::Black),
        ];

        for (piece_idx, kind, color) in PIECE_MAP {
            let mut bb = self.pieces[piece_idx];
            while !bb.is_empty(){
                let sq = bb.pop_lsb();
                f(sq, Piece { kind, color });
            }
        }
    }
}
