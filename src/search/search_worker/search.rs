use crate::board::transposition_table::EntryFlag;
use crate::globals;
use crate::globals::{INFINITY, MATE_SCORE};
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;

impl<'a> SearchWorker<'a> {
    pub(crate) fn negamax(&mut self, depth: i32, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if self.is_time_up() {
            return 0;
        }

        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }

        if (ply > 0 && self.state.is_repetition()) || self.state.halfmove_clock >= 100 {
            return 0;
        }

        if depth <= 0 {
            return self.quiescence(alpha, beta, ply);
        }

        let in_check = self.state.is_in_check(self.state.board.side_to_move);
        let is_pv_node = beta - alpha > 1;
        self.nodes += 1;
        let orig_alpha = alpha;

        let static_eval = self.evaluate(ply);
        self.stack[ply].eval = static_eval;

        let mut tt_move = None;
        if let Some(entry) = self.tt_probe() {
            if let Some(score) = entry.cutoff_score(depth, alpha, beta, ply) {
                return score;
            }
            tt_move = Some(entry.best_move());
        }

        // Pre move pruning
        if let Some(score) =
            self.try_pre_move_pruning(depth, ply, beta, in_check, tt_move.is_some(), static_eval)
        {
            return score;
        }

        // Setup Move Loop
        let mut picker = MovePicker::new(
            tt_move,
            self.stack[ply].killers[0],
            self.stack[ply].killers[1],
        );
        let mut max_eval = -INFINITY;
        let mut best_move = None;
        let mut legal_moves_played = 0;

        // Iterate over each move
        while let Some(mv) = picker.next_move(self) {
            if !self.make_move(mv, ply) {
                continue;
            }

            legal_moves_played += 1;
            let is_quiet = mv.captured().is_none() && !mv.is_promotion();
            let gives_check = self.state.is_in_check(self.state.board.side_to_move);

            // Apply late move reduction
            let eval = self.search_single_move(
                depth,
                ply,
                alpha,
                beta,
                legal_moves_played,
                is_quiet,
                gives_check,
                in_check,
            );

            self.undo_move(mv);

            if self.is_time_up() {
                return 0;
            }

            if eval > max_eval {
                max_eval = eval;
                best_move = Some(mv);
            }
            alpha = alpha.max(eval);

            if alpha >= beta {
                if is_quiet {
                    self.update_killers(ply, mv);
                    self.update_history(
                        self.state.board.side_to_move as usize,
                        mv.from() as usize,
                        mv.to() as usize,
                        depth,
                    );
                }
                break;
            }
        }

        // Stalemate / Check
        if legal_moves_played == 0 {
            return if in_check { -INFINITY + ply as i32 } else { 0 };
        }

        if let Some(mv) = best_move {
            self.tt_store(orig_alpha, beta, depth as u32, ply, mv, max_eval);
        }

        max_eval
    }

    /// Tactical Quiescence Search
    fn quiescence(&mut self, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.nodes += 1;
        if self.is_time_up() {
            return 0;
        }
        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }

        let alpha_orig = alpha;

        // Probe TT
        let mut tt_move = None;
        if let Some(entry) = self.tt_probe() {
            if let Some(score) = entry.cutoff_score(0, alpha, beta, ply) {
                return score;
            }
            tt_move = Some(entry.best_move());
        }

        let in_check = self.state.is_in_check(self.state.board.side_to_move);
        let static_eval;
        let mut best_move = None;
        let mut best_value;

        if in_check {
            static_eval = 0;
            best_value = -MATE_SCORE + ply as i32; // mated unless an evasion exists
        } else {
            // Stand pat
            static_eval = self.evaluate(ply);
            best_value = static_eval;
            if best_value >= beta {
                return best_value;
            }
            alpha = alpha.max(best_value);
        }

        let mut picker = if in_check {
            MovePicker::new(tt_move, None, None) // In check all possible moves are evaluated
        } else {
            MovePicker::new_quiescence(tt_move)
        };

        while let Some(mv) = picker.next_move(self) {
            if !in_check {
                // Static exchange evaluation
                const SEE_MARGIN: i32 = 200;
                let threshold = (alpha - static_eval - SEE_MARGIN).max(0);
                if !mv.is_promotion() && !self.state.is_move_greater_equal(mv, threshold) {
                    continue;
                }
            }

            if !self.make_move(mv, ply) {
                continue; // Illegal move continue
            }
            let score = -self.quiescence(-beta, -alpha, ply + 1);
            self.undo_move(mv);
            if self.is_time_up() {
                return 0;
            }

            if score > best_value {
                best_value = score;
                best_move = Some(mv);
                if score > alpha {
                    alpha = score;
                    if score >= beta {
                        break;
                    }
                }
            }
        }
        if let Some(mv) = best_move {
            self.tt_store(alpha_orig, beta, 0, ply, mv, best_value);
        }
        best_value
    }

    pub fn search_root(
        &mut self,
        depth: i32,
        previous_best_move: Option<Move>,
    ) -> Option<(Move, i32)> {
        let mut picker = MovePicker::new(previous_best_move, None, None); // Initialize the move picker

        let mut max_eval = -INFINITY; // The score of the best move found so far
        let mut best_move = None; // The best move found so far
        let mut alpha = -INFINITY;
        let beta = INFINITY;

        while let Some(mv) = picker.next_move(self) {
            if !self.make_move(mv, 0) {
                continue;
            }

            if best_move.is_none() {
                best_move = Some(mv);
            }

            let eval = -self.negamax(depth - 1, 1, -beta, -alpha);
            self.undo_move(mv);

            if self.is_time_up() {
                if previous_best_move.is_none() {
                    break;
                }
                return None;
            }

            if eval > max_eval {
                max_eval = eval;
                best_move = Some(mv);
                alpha = eval;
            }
        }

        if let Some(mv) = best_move {
            self.table.store(
                // Store in TT
                self.state.hash,
                depth as u32,
                max_eval,
                EntryFlag::Exact,
                mv,
                0,
            );
            Some((mv, max_eval))
        } else {
            None
        }
    }
}
