#[cfg(test)]
mod tests {
    use ruebli::board::fen_parser::{parse_fen, ParseError};
    use ruebli::types::{piece, CASTLE_BLACK_QUEEN, CASTLE_WHITE_KING, CASTLE_WHITE_QUEEN, CASTLE_BLACK_KING};
    use ruebli::types::piece::PieceColor;
    //-------------
    // Invalid fens
    //-------------

    #[test]
    fn test_missing_fields() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidFieldCount {
                expected: 6,
                found: 3
            })
        );
    }

    #[test]
    fn test_empty() {
        let fen = "";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidFieldCount {
                expected: 6,
                found: 0
            })
        );
    }

    #[test]
    fn test_no_piece_data() {
        let fen = "/////// w KQkq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidFileCount {
                rank: 7,
                expected: 8,
                found: 0
            })
        );
    }

    #[test]
    fn test_invalid_en_passant() {
        let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e4 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidEnPassantSquare(String::from("e4")))
        );
    }

    #[test]
    fn test_invalid_rank_count() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidRankCount {
                expected: 8,
                found: 9
            })
        );
    }

    #[test]
    fn test_invalid_piece_character() {
        let fen = "rnbqkbnr/pppppppp/8/8/4X3/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidPieceChar('X'))
        );
    }

    #[test]
    fn test_invalid_empty_square_digit() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/08/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidPieceChar('0'))
        );
    }

    #[test]
    fn test_invalid_active_color() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidSideToMove("x".to_string()))
        );
    }

    #[test]
    fn test_invalid_castling_rights() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQXq - 0 1";
        assert_eq!(
            parse_fen(String::from(fen)),
            Err(ParseError::InvalidCastlingRights("KQXq".to_string()))
        );
    }

    //-----------
    // Valid fens
    //-----------

    #[test]
    fn test_startpos_fen() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let result = parse_fen(String::from(fen)).unwrap();

        assert_eq!(result.board.side_to_move, PieceColor::White);
        assert_eq!(
            result.castling_rights,
            CASTLE_BLACK_KING | CASTLE_WHITE_QUEEN | CASTLE_BLACK_QUEEN | CASTLE_WHITE_KING
        );
        assert_eq!(result.halfmove_clock, 0);
        assert_eq!(result.fullmove_number, 1);
        assert_eq!(result.en_passant, None);

        assert_eq!(result.board.pieces[piece::W_PAWN].0, 65_280);
        assert_eq!(result.board.pieces[piece::W_ROOK].0, 129);
        assert_eq!(result.board.pieces[piece::W_KNIGHT].0, 66);
        assert_eq!(result.board.pieces[piece::W_BISHOP].0, 36);
        assert_eq!(result.board.pieces[piece::W_QUEEN].0, 8);
        assert_eq!(result.board.pieces[piece::W_KING].0, 16);

        assert_eq!(result.board.pieces[piece::B_PAWN].0, 65_280 << 40);
        assert_eq!(result.board.pieces[piece::B_ROOK].0, 129 << 56);
        assert_eq!(result.board.pieces[piece::B_KNIGHT].0, 66 << 56);
        assert_eq!(result.board.pieces[piece::B_BISHOP].0, 36 << 56);
        assert_eq!(result.board.pieces[piece::B_QUEEN].0, 8 << 56);
        assert_eq!(result.board.pieces[piece::B_KING].0, 16 << 56);
    }

    #[test]
    fn test_active_black() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
        let result = parse_fen(String::from(fen)).unwrap();

        assert_eq!(result.board.side_to_move, PieceColor::Black);
    }

    #[test]
    fn test_no_castling_fen() {
        let fen = "rnb2bnr/pp1ppppp/8/2p2qP1/3k4/1Q4K1/PPPPPP1P/RNB2BNR b - - 0 1";
        let result = parse_fen(String::from(fen)).unwrap();
        assert_eq!(0, result.castling_rights);
    }

    #[test]
    fn test_en_passant_fen() {
        let fen = "rnbqkbnr/1ppppppp/8/p7/8/8/PPPPPPPP/RNBQKBNR b KQkq e3 0 1";

        let result = parse_fen(String::from(fen)).unwrap();
        assert_eq!(result.en_passant.unwrap(), 20);
    }
}
