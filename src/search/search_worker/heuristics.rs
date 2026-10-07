use crate::board::transposition_table::{EntryFlag, TTRead};
use crate::search::lmr::get_lmr;
use crate::search::search_worker::core::SearchWorker;
use crate::types::piece::PieceColor;
use crate::types::{Move, piece};

impl SearchWorker<'_> {

    #[inline]
    pub fn update_killers(&mut self, ply: usize, mv: Move) {
        if self.stack[ply].killers[0] != Some(mv) {
            self.stack[ply].killers[1] = self.stack[ply].killers[0];
            self.stack[ply].killers[0] = Some(mv);
        }
    }

    #[inline]
    pub(crate) fn tt_store(
        &mut self,
        orig_alpha: i32,
        beta: i32,
        depth: u32,
        ply: usize,
        mv: Move,
        score: i32,
    ) {
        let flag = if score >= beta {
            EntryFlag::LowerBound
        } else if score <= orig_alpha {
            EntryFlag::UpperBound
        } else {
            EntryFlag::Exact
        };

        let hash = self.state.hash;
        self.table.store(hash, depth, score, flag, mv, ply)
    }

    #[inline]
    pub(crate) fn tt_probe(&self) -> Option<TTRead> {
        if let Some(entry) = self.table.probe(self.state.hash) {
            return Some(entry);
        }
        None
    }

    #[inline]
    pub(crate) fn try_pre_move_pruning(
        &mut self,
        depth: i32,
        ply: usize,
        beta: i32,
        in_check: bool,
        has_tt_move: bool,
        static_eval: i32,
    ) -> Option<i32> {
        // Reverse Futility Pruning
        if has_tt_move && !in_check {
            let margin = 150 * depth;
            if static_eval >= beta + margin {
                return Some(static_eval);
            }
        }

        // Null Move Pruning
        if depth >= 2 && !in_check && ply > 0 {
            let us = self.state.board.side_to_move;
            let pawns = if us == PieceColor::White {
                self.state.board.pieces[piece::W_PAWN]
            } else {
                self.state.board.pieces[piece::B_PAWN]
            };
            let kings = if us == PieceColor::White {
                self.state.board.pieces[piece::W_KING]
            } else {
                self.state.board.pieces[piece::B_KING]
            };

            if (self.state.friendly_pieces().0 ^ pawns.0 ^ kings.0) != 0 {
                self.make_null_move(ply);
                let null_eval = -self.negamax(depth - 1 - 4, ply + 1, -beta, -beta + 1);
                self.undo_null_move();

                if null_eval >= beta {
                    return Some(beta);
                }
            }
        }
        None
    }

    /// Handles Late Move Reductions
    #[inline]
    pub(crate) fn search_single_move(
        &mut self,
        depth: i32,
        ply: usize,
        alpha: i32,
        beta: i32,
        moves_played: usize,
        is_quiet: bool,
        gives_check: bool,
        in_check: bool,
    ) -> i32 {
        let mut eval;

        // Apply LMR
        if moves_played >= 3 && !in_check && depth >= 2 && is_quiet && !gives_check {
            let reduction = get_lmr(depth, moves_played);

            // Zero-window reduced search
            eval = -self.negamax(depth - 1 - reduction, ply + 1, -alpha - 1, -alpha);

            // Re-search at full depth if it beats alpha
            if eval > alpha {
                eval = -self.negamax(depth - 1, ply + 1, -beta, -alpha);
            }
        } else {
            // Full depth search
            eval = -self.negamax(depth - 1, ply + 1, -beta, -alpha);
        }

        eval
    }
}
