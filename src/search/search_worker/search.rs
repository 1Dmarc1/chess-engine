use crate::board::transposition_table::EntryFlag;
use crate::globals;
use crate::globals::{INFINITY, MATE_SCORE};
use crate::move_gen::MoveList;
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;

const SEE_MARGIN: i32 = 200;

pub struct SearchState {
    pub max_eval: i32,
    pub best_move: Option<Move>,
    pub legal_moves_played: usize,
    pub quiet_moves_count: usize,
    pub failed_quiets: MoveList,
    pub failed_captures: MoveList,
}

pub struct MoveContext {
    pub mv: Move,
    pub gives_check: bool,
    pub extension: i32,
}

impl MoveContext {
    pub fn new(mv: Move, gives_check : bool) -> MoveContext {
        MoveContext{
            mv,
            gives_check,
            extension: 0,
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            max_eval: -INFINITY,
            best_move: None,
            legal_moves_played: 0,
            quiet_moves_count: 0,
            failed_quiets: MoveList::default(),
            failed_captures: MoveList::default(),
        }
    }
}


impl<'a> SearchWorker<'a> {


    /// Tactical Quiescence Search
    pub(crate) fn quiescence(&mut self, mut alpha: i32, beta: i32, ply: usize) -> i32 {
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

    pub(crate) fn search_root(
        &mut self,
        depth: i32,
        previous_best_move: Option<Move>,
        mut alpha: i32,
        beta: i32
    ) -> Option<(Move, i32)> {

        let mut picker = MovePicker::new(previous_best_move, None, None, 0);
        let mut max_eval = -INFINITY;
        let mut best_move = None;
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
            self.tt_store(orig_alpha, beta, depth as u32, 0, mv, max_eval);
            Some((mv, max_eval))
        } else {
            None
        }
    }
}
