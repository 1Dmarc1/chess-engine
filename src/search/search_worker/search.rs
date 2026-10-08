use crate::board::transposition_table::EntryFlag;
use crate::globals;
use crate::globals::{INFINITY, MATE_SCORE};
use crate::move_gen::MoveList;
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;

const HIST_PRUNE_MARGIN: i32 = 1500;
const HIST_PRUNE_MAX_DEPTH: i32 = 2;

impl<'a> SearchWorker<'a> {
    pub(crate) fn negamax(&mut self, mut depth: i32, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if self.is_time_up() {
            return 0;
        }

        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }

        if (ply > 0 && self.state.is_repetition()) || self.state.halfmove_clock >= 100 {
            return 0;
        }

        // Check extension
        let in_check = self.state.is_in_check(self.state.board.side_to_move);
        if in_check{
            depth += 1;
        }

        if depth <= 0 {
            return self.quiescence(alpha, beta, ply);
        }


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

        // Reduce depth if no tt_move was found
        if depth >= 4 && tt_move.is_none() {
            depth -= 1;
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
            ply
        );
        let mut max_eval = -INFINITY;
        let mut best_move = None;
        let mut legal_moves_played = 0;
        let mut quiet_moves_count = 0;

        let mut failed_quiet_moves: MoveList = MoveList::default();
        let mut failed_capture_moves: MoveList = MoveList::default();
        
        let improving = if !in_check && ply >= 2 {
            static_eval > self.stack[ply - 2].eval
        } else {
            true
        };

        // Iterate over each move
        while let Some(mv) = picker.next_move(self) {
            if !self.make_move(mv, ply) {
                continue;
            }

            legal_moves_played += 1;
            let is_quiet = mv.captured().is_none() && !mv.is_promotion();
            let gives_check = self.state.is_in_check(self.state.board.side_to_move);

            if !is_pv_node && !in_check && is_quiet && !gives_check && best_move.is_some() {
                // History pruning
                if depth <= HIST_PRUNE_MAX_DEPTH {
                    let hist_score = self.history.score_quiet_move(mv, ply, &self.stack);
                    if hist_score < -HIST_PRUNE_MARGIN * depth {
                        self.undo_move(mv);
                        continue;
                    }
                }

                // Late move pruning
                if depth <= 4{
                    let lmp_threshold = 3 + 2 * depth * depth;
                    let limit = if improving { lmp_threshold } else { lmp_threshold / 2 }.max(4);

                    if quiet_moves_count >= limit {
                        self.undo_move(mv);
                        continue; // Skip searching this late, low-probability quiet move entirely!
                    }
                }

            }

            // Only increment if it's a quiet move that survived pruning
            if is_quiet {
                quiet_moves_count += 1;
            }

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
                    self.history.update_quiet_move_history(
                        depth,
                        ply,
                        mv,
                        self.state.board.side_to_move as usize,
                        &self.stack,
                        failed_quiet_moves.as_mut_slice()
                    );
                }
                else if mv.captured().is_some() {
                    self.history.update_capture_move_history(mv, depth, failed_capture_moves.as_mut_slice());
                }
                break;
            }
            else {
                if is_quiet{
                    failed_quiet_moves.push(mv);
                }
                else{
                    failed_capture_moves.push(mv);
                }
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
            MovePicker::new(tt_move, None, None, ply) // In check all possible moves are evaluated
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
        mut alpha: i32,
        beta: i32
    ) -> Option<(Move, i32)> {

        let mut picker = MovePicker::new(previous_best_move, None, None, 0);
        let mut max_eval = -INFINITY;
        let mut best_move = None;

        // Capture the original alpha to determine correct TT flags later
        let orig_alpha = alpha;

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
            }

            alpha = alpha.max(eval);

            // Beta cutoff
            if alpha >= beta {
                break;
            }
        }

        if let Some(mv) = best_move {
            let flag = if max_eval >= beta {
                EntryFlag::LowerBound // We failed high
            } else if max_eval <= orig_alpha {
                EntryFlag::UpperBound // We failed low
            } else {
                EntryFlag::Exact      // We stayed inside the window
            };

            self.table.store(
                self.state.hash,
                depth as u32,
                max_eval,
                flag,
                mv,
                0
            );
            Some((mv, max_eval))
        } else {
            None
        }
    }
}
