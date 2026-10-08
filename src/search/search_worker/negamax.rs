use crate::globals;
use crate::globals::MATE_SCORE;
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::core::SearchWorker;
use crate::search::search_worker::search::{MoveContext, SearchState};
use crate::types::Move;

impl SearchWorker<'_> {
    pub(crate) fn negamax(&mut self, mut depth: i32, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if self.is_time_up()
            || ply > 0 && self.state.is_repetition()
            || self.state.halfmove_clock >= 100
        {
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
        let mut picker = MovePicker::new(
            tt_move,
            self.stack[ply].killers[0],
            self.stack[ply].killers[1],
            ply,
        );

        self.stack[ply].improving = if !in_check && ply >= 2 {
            static_eval > self.stack[ply - 2].static_eval
        } else {
            true
        };

        // Iterate over each move
        let mut search_state: SearchState = SearchState::default();
        while let Some(mv) = picker.next_move(self) {
            if !self.make_move(mv, ply) {
                continue;
            }
            search_state.legal_moves_played += 1;

            let gives_check = self.state.is_in_check(self.state.board.side_to_move);
            let move_context = MoveContext::new(mv, gives_check);

            if mv.is_quiet() {
                if !gives_check
                    && search_state.best_move.is_some()
                    && self.should_prune_quiet_move(depth, mv, ply, &search_state)
                {
                    self.undo_move(mv);
                    continue;
                }
                // Only increment if it's a quiet move that survived pruning
                search_state.quiet_moves_count += 1;
            }

            // Apply late move reduction
            let eval = self.search_single_move(
                depth,
                ply,
                alpha,
                beta,
                search_state.legal_moves_played,
                &move_context,
            );

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
            return if in_check {
                -MATE_SCORE + ply as i32
            } else {
                0
            };
        }

        if let Some(mv) = search_state.best_move {
            self.tt_store(
                orig_alpha,
                beta,
                depth as u32,
                ply,
                mv,
                search_state.max_eval,
            );
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
            self.history.update_capture_move_history(
                mv,
                depth,
                state.failed_captures.as_mut_slice(),
            );
        }
    }
}
