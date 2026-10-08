use crate::globals;
use crate::globals::{MATE_SCORE, MIN_MATE_SCORE};
use crate::search::lmr::get_lmr;
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::core::SearchWorker;
use crate::search::search_worker::search::{MoveContext, SearchState};
use crate::types::piece::PieceColor;
use crate::types::{Move, piece};

const LMR_MIN_MOVES: usize = 3;
const LMR_MIN_DEPTH: i32 = 2;
const HIST_PRUNE_MARGIN: i32 = 1500;
const HIST_PRUNE_MAX_DEPTH: i32 = 2;
const RFP_MAX_DEPTH: i32 = 8;
const RFP_MARGIN_MULTIPLIER: i32 = 150;
const NMP_MIN_DEPTH: i32 = 2;
const NMP_REDUCTION: i32 = 4;

impl SearchWorker<'_> {
    pub(crate) fn negamax(&mut self, mut depth: i32, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if self.is_time_up() || ply > 0 && self.state.is_repetition() || self.state.halfmove_clock >= 100 {
            return 0;
        }

        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }

        let in_check = self.state.is_in_check(self.state.board.side_to_move);
        self.stack[ply].in_check = in_check;

        // Check extension
        if in_check {
            depth += 1;
        }

        if depth <= 0 {
            return self.quiescence(alpha, beta, ply);
        }

        self.nodes += 1;
        let orig_alpha = alpha;

        let static_eval = self.evaluate(ply);
        self.stack[ply].static_eval = static_eval;

        let is_pv_node = beta - alpha > 1;
        self.stack[ply].is_pv_node = is_pv_node;

        let mut tt_move = None;
        if let Some(entry) = self.tt_probe() {
            if let Some(score) = entry.cutoff_score(depth, alpha, beta, ply) {
                return score;
            }
            tt_move = Some(entry.best_move());
        }

        // Reduce depth if no tt_move was found
        if depth >= 4 && tt_move.is_none() {
            depth -= 1;
        }

        // Pre move pruning
        if let Some(score) = self.try_pre_move_pruning(depth, ply, beta) {
            return score;
        }

        // Setup Move Loop
        let mut picker = MovePicker::new(tt_move, self.stack[ply].killers[0], self.stack[ply].killers[1], ply);

        self.stack[ply].improving = if !in_check && ply >= 2 {
            static_eval > self.stack[ply - 2].static_eval
        } else {
            true
        };

        // Iterate over each move
        let mut search_state: SearchState = SearchState::default();
        while let Some(mv) = picker.next_move(self) {

            // SEE pruning
            if depth <= 4 && search_state.best_move.is_some() && !self.stack[ply].in_check {
                // Require losing more material at deeper depths before pruning
                let see_margin = if mv.is_quiet() { -50 * depth } else { -100 * depth };

                if !self.state.is_move_greater_equal(mv, see_margin) {
                    //continue;
                }
            }

            if !self.make_move(mv, ply) {
                continue;
            }
            search_state.legal_moves_played += 1;

            let gives_check = self.state.is_in_check(self.state.board.side_to_move);
            let move_context = MoveContext::new(mv, gives_check);

            if search_state.best_move.is_some() && self.should_prune_move(depth, mv, ply, &search_state, &move_context) {
                self.undo_move(mv);
                continue;
            }
            if mv.is_quiet() {
                search_state.quiet_moves_count += 1;
            }

            // Apply late move reduction
            let eval = self.search_single_move(depth, ply, alpha, beta, search_state.legal_moves_played, &move_context);

            self.undo_move(mv);

            if self.is_time_up() {
                return 0;
            }

            if eval > search_state.max_eval {
                search_state.max_eval = eval;
                search_state.best_move = Some(mv);
            }
            alpha = alpha.max(eval);

            if alpha >= beta {
                self.handle_beta_cutoff(mv, depth, ply, &mut search_state);
                break;
            } else {
                if mv.is_quiet() {
                    search_state.failed_quiets.push(mv);
                } else {
                    search_state.failed_captures.push(mv);
                }
            }
        }

        // Stalemate / Check
        if search_state.legal_moves_played == 0 {
            return if in_check { -MATE_SCORE + ply as i32 } else { 0 };
        }

        if let Some(mv) = search_state.best_move {
            self.tt_store(orig_alpha, beta, depth as u32, ply, mv, search_state.max_eval);
        }

        search_state.max_eval
    }

    #[inline]
    fn handle_beta_cutoff(&mut self, mv: Move, depth: i32, ply: usize, state: &mut SearchState) {
        if mv.is_quiet() {
            self.update_killers(ply, mv);
            self.history.update_quiet_move_history(
                depth,
                ply,
                mv,
                self.state.board.side_to_move as usize,
                &self.stack,
                state.failed_quiets.as_mut_slice(),
            );
        } else if mv.captured().is_some() {
            self.history.update_capture_move_history(mv, depth, state.failed_captures.as_mut_slice());
        }
    }

    /// Searches a single move and returns its score.
    #[inline]
    fn search_single_move(&mut self, depth: i32, ply: usize, alpha: i32, beta: i32, moves_played: usize, mv_ctx: &MoveContext) -> i32 {
        let mut eval;
        let in_check = self.stack[ply].in_check;

        // Apply LMR
        if moves_played >= LMR_MIN_MOVES && !in_check && depth >= LMR_MIN_DEPTH && mv_ctx.mv.is_quiet() && !mv_ctx.gives_check {
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

    #[inline]
    pub(crate) fn should_prune_move(&mut self, depth: i32, mv: Move, ply: usize, search_state: &SearchState, context: &MoveContext) -> bool {
        let mut res = false;
        let stack_entry = self.stack[ply];
        if stack_entry.is_pv_node || stack_entry.in_check || context.gives_check {
            return false;
        }
        let is_quiet = mv.is_quiet();

        if is_quiet {
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
        }
        res
    }

    #[inline]
    pub(crate) fn try_pre_move_pruning(&mut self, depth: i32, ply: usize, beta: i32) -> Option<i32> {
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
}
