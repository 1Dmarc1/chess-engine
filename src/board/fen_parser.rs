use crate::board::board_state::BoardState;
use crate::board::game_state::GameState;
use crate::types::piece::PieceColor;
use crate::types::{
    CASTLE_BLACK_KING, CASTLE_BLACK_QUEEN, CASTLE_WHITE_KING, CASTLE_WHITE_QUEEN, piece,
};
use crate::util;

/// Represent errors that can occur while parsing a FEN string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// FEN string does not contain all 6 required fields separated by spaces.
    InvalidFieldCount { expected: usize, found: usize },

    /// The piece placement section does not contain exactly 8 ranks.
    InvalidRankCount { expected: u8, found: u8 },

    /// A rank does not sum up to 8 squares (contains too few or too many files/empty counts).
    InvalidFileCount { rank: u8, expected: u8, found: u8 },

    /// Encountered an unrecognized piece character or invalid token.
    InvalidPieceChar(char),

    /// Active color field is not 'w' or 'b'.
    InvalidSideToMove(String),

    /// Castling rights contain invalid characters or duplicate flags.
    InvalidCastlingRights(String),

    /// En passant target is not a valid square string
    InvalidEnPassantSquare(String),

    /// Halfmove clock is not a valid non-negative integer.
    InvalidHalfmoveClock(String),

    /// Fullmove number is not a valid positive integer
    InvalidFullmoveNumber(String),

    /// The FEN position violates essential chess rules.
    InvalidKingCount { piece: char, count: usize },
}

pub fn parse_fen(fen: String) -> Result<GameState, ParseError> {
    let mut iter = fen.split_ascii_whitespace();

    let piece_placement_data_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 0,
    })?;
    let active_color_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 1,
    })?;
    let castling_availability_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 2,
    })?;
    let en_passant_target_square_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 3,
    })?;
    let halfmove_clock_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 4,
    })?;
    let fullmove_number_str = iter.next().ok_or(ParseError::InvalidFieldCount {
        expected: 6,
        found: 5,
    })?;

    let mut board: BoardState = BoardState::new();
    parse_piece_data(piece_placement_data_str, &mut board)?;

    // Handle side to move
    let side_to_move: PieceColor = match active_color_str {
        "w" => PieceColor::White,
        "b" => PieceColor::Black,
        _ => return Err(ParseError::InvalidSideToMove(active_color_str.to_string())),
    };
    board.side_to_move = side_to_move;

    // Handle castling
    let castle_availability = castling_availability_str.chars();
    let mut castling_rights: u8 = 0;
    for char in castle_availability {
        match char {
            'K' => castling_rights |= CASTLE_WHITE_KING,
            'Q' => castling_rights |= CASTLE_WHITE_QUEEN,
            'k' => castling_rights |= CASTLE_BLACK_KING,
            'q' => castling_rights |= CASTLE_BLACK_QUEEN,
            '-' => (),
            _ => {
                return Err(ParseError::InvalidCastlingRights(String::from(
                    castling_availability_str,
                )));
            }
        }
    }

    // Handle en passant
    let en_passant_target_square = util::algebraic_notation_to_square(en_passant_target_square_str);
    if let Some(square) = en_passant_target_square {
        if board.side_to_move == PieceColor::Black && !(16..=23).contains(&square) {
            return Err(ParseError::InvalidEnPassantSquare(String::from(
                en_passant_target_square_str,
            )));
        } else if board.side_to_move == PieceColor::White && !(40..=47).contains(&square) {
            return Err(ParseError::InvalidEnPassantSquare(String::from(
                en_passant_target_square_str,
            )));
        }
    }

    // Handle half move and fullmove
    let half_move_clock = halfmove_clock_str
        .parse()
        .map_err(|_| ParseError::InvalidHalfmoveClock(halfmove_clock_str.to_string()))?;
    let fullmove: u32 = fullmove_number_str
        .parse()
        .map_err(|_| ParseError::InvalidFullmoveNumber(fullmove_number_str.to_string()))?;

    let result: GameState = GameState::new(
        board,
        en_passant_target_square,
        castling_rights,
        half_move_clock,
        fullmove,
    );
    Ok(result)
}

fn parse_piece_data(data: &str, board: &mut BoardState) -> Result<(), ParseError> {
    let mut piece_index: usize;
    let mut file: u8 = 0;
    let mut rank: u8 = 7;
    for char in data.chars() {
        if file > 8 {
            return Err(ParseError::InvalidFileCount {
                rank,
                expected: 8,
                found: file,
            });
        }

        match char {
            'P' => piece_index = piece::W_PAWN,
            'N' => piece_index = piece::W_KNIGHT,
            'B' => piece_index = piece::W_BISHOP,
            'R' => piece_index = piece::W_ROOK,
            'Q' => piece_index = piece::W_QUEEN,
            'K' => piece_index = piece::W_KING,
            'p' => piece_index = piece::B_PAWN,
            'n' => piece_index = piece::B_KNIGHT,
            'b' => piece_index = piece::B_BISHOP,
            'r' => piece_index = piece::B_ROOK,
            'q' => piece_index = piece::B_QUEEN,
            'k' => piece_index = piece::B_KING,
            '/' => {
                if file != 8 {
                    return Err(ParseError::InvalidFileCount {
                        rank,
                        expected: 8,
                        found: file,
                    });
                }
                if rank == 0 {
                    return Err(ParseError::InvalidRankCount {
                        expected: 8,
                        found: 9,
                    });
                }
                rank -= 1;
                file = 0;
                continue;
            }
            '1'..='8' => {
                let offset: u8 = char.to_digit(10).unwrap() as u8;
                file += offset;
                continue;
            }
            _ => return Err(ParseError::InvalidPieceChar(char)),
        }
        board.pieces[piece_index].set(util::rank_file_to_square(rank, file));
        file += 1;
    }

    // Verify all ranks and files have been parsed.
    if rank != 0 || file != 8 {
        return Err(ParseError::InvalidRankCount {
            expected: 8,
            found: 8 - rank,
        });
    }

    Ok(())
}
