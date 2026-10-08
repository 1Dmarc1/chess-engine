use crate::board::transposition_table::{EntryFlag, TTRead};
use crate::globals::MIN_MATE_SCORE;
use crate::search::lmr::get_lmr;
use crate::search::search_worker::core::SearchWorker;
use crate::search::search_worker::search::SearchState;
use crate::types::piece::PieceColor;
use crate::types::{Move, piece};

const RFP_MAX_DEPTH: i32 = 8;
const RFP_MARGIN_MULTIPLIER: i32 = 150;

const NMP_MIN_DEPTH: i32 = 2;
const NMP_REDUCTION: i32 = 4;

const LMR_MIN_MOVES: usize = 3;
const LMR_MIN_DEPTH: i32 = 2;

const HIST_PRUNE_MARGIN: i32 = 1500;
const HIST_PRUNE_MAX_DEPTH: i32 = 2;

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
    pub(crate) fn should_prune_quiet_move(&mut self, depth : i32, mv : Move, ply : usize, search_state: &SearchState) -> bool{
        let mut res = false;
        let stack_entry = self.stack[ply];
        if stack_entry.is_pv_node || stack_entry.in_check{
            return false;
        }

        // History pruning
        if depth <= HIST_PRUNE_MAX_DEPTH {
            let hist_score = self.history.score_quiet_move(mv, ply, &self.stack);
            if hist_score < -HIST_PRUNE_MARGIN * depth {
                res = true;
            }
        }

        // Late move pruning
        if !res && depth <= 4 {
            let lmp_threshold = 3 + 2 * depth * depth;
            let limit = if self.stack[ply].improving { lmp_threshold } else { lmp_threshold / 2 }.max(4);

            if search_state.quiet_moves_count >= limit as usize {
                res = true;
            }
        }
        res
    }

    #[inline]
    pub(crate) fn try_pre_move_pruning(
        &mut self,
        depth: i32,
        ply: usize,
        beta: i32,
    ) -> Option<i32> {
        let static_eval = self.stack[ply].static_eval;
        let in_check = self.stack[ply].in_check;

        // Reverse Futility Pruning
        if !self.stack[ply].is_pv_node && depth <= RFP_MAX_DEPTH && !in_check
            && beta > -MIN_MATE_SCORE      // never prune when the window is in mate range
            && static_eval.abs() < MIN_MATE_SCORE
        {
            let margin = RFP_MARGIN_MULTIPLIER * depth;
            if static_eval >= beta + margin {
                return Some(static_eval);
            }
        }

        // Null Move Pruning
        if depth >= NMP_MIN_DEPTH && !in_check && ply > 0 {
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
                let null_eval = -self.negamax(depth - 1 - NMP_REDUCTION, ply + 1, -beta, -beta + 1);
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
    ) -> i32 {
        let mut eval;
        let in_check = self.stack[ply].in_check;

        // Apply LMR
        if moves_played >= LMR_MIN_MOVES
            && !in_check
            && depth >= LMR_MIN_DEPTH
            && is_quiet
            && !gives_check
        {
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
