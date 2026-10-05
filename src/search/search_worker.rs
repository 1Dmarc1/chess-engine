use crate::board::game_state::GameState;
use crate::board::transposition_table::{EntryFlag, TranspositionTable};
use crate::globals;
use crate::globals::INFINITY;
use crate::search::evaluation;
use crate::search::move_picker::MovePicker;
use crate::types::piece::PieceColor;
use crate::types::{Move, piece};
use nnue_rs::{Accumulator, Board, Network};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::search::lmr::get_lmr;

#[derive(Clone, Copy, Default)]
pub struct StackEntry {
    /// The two killer moves found at this depth.
    pub killers: [Option<Move>; 2],
}

pub struct SearchWorker<'a> {
    pub state: GameState,
    pub table: Arc<TranspositionTable>,
    pub network: &'a Network,
    pub should_stop: Arc<AtomicBool>,

    pub stack: [StackEntry; globals::MAX_SEARCH_PLY],

    pub accumulators: [Accumulator; globals::MAX_SEARCH_PLY], // On accumulator for each search depth
    pub accumulator_map : [usize; globals::MAX_SEARCH_PLY], // Defines for which ply which accumulator should be accessed

    pub history: [[[i32; 64]; 64]; 2], // [Color][FromSquare][ToSquare]

    pub nodes: u64,
}

impl<'a> SearchWorker<'a> {
    pub fn new(
        state: GameState,
        table: Arc<TranspositionTable>,
        network: &'a Network,
        should_stop: Arc<AtomicBool>,
    ) -> Self {
        let mut accumulators = std::array::from_fn(|_| network.empty_accumulator());
        accumulators[0] = network.accumulator(&state.board);

        Self {
            state,
            table,
            network,
            should_stop,
            stack: [StackEntry::default(); globals::MAX_SEARCH_PLY],
            accumulators,
            accumulator_map: std::array::from_fn(|i| i),
            history: [[[0; 64]; 64]; 2],
            nodes: 0,
        }
    }

    #[inline(always)]
    pub fn make_move(&mut self, mv: Move, ply: usize) -> bool {
        let parent_board = self.state.board;
        if !self.state.make_move_if_legal(&mv, &self.table.zobrist) {
            return false;
        }
        let left = self.accumulator_map[ply]; // The current accumulator
        self.accumulator_map[ply + 1] = ply + 1; // Ensure that after a potential null move the correct accumulator is used.


        let (lower_half, upper_half) = self.accumulators.split_at_mut(ply + 1);
        let base_acc = &lower_half[left];
        let mut target_acc = &mut upper_half[0];

        self.network
            .update(&parent_board, &self.state.board, &base_acc, &mut target_acc);
        true
    }

    #[inline(always)]
    pub fn undo_move(&mut self, mv: Move) {
        self.state.undo_move(&mv);
    }

    #[inline(always)]
    pub fn make_null_move(&mut self, ply: usize) {
        self.state.make_null_move(&self.table.zobrist);
        self.accumulator_map[ply + 1] = self.accumulator_map[ply];
    }

    #[inline(always)]
    pub fn undo_null_move(&mut self) {
        self.state.undo_null_move();
    }

    #[inline(always)]
    pub fn evaluate(&self, ply: usize) -> i32 {
        let acc_idx = self.accumulator_map[ply];
        self.network
            .evaluate_accumulator(&self.accumulators[acc_idx], self.state.board.side_to_move())
    }

    #[inline(always)]
    pub fn update_killers(&mut self, ply: usize, mv: Move) {
        if self.stack[ply].killers[0] != Some(mv) {
            self.stack[ply].killers[1] = self.stack[ply].killers[0];
            self.stack[ply].killers[0] = Some(mv);
        }
    }

    #[inline(always)]
    pub fn update_history(&mut self, color_idx: usize, from: usize, to: usize, depth: i32) {
        let bonus = depth * depth;
        self.history[color_idx][from][to] += bonus;
    }

    #[inline(always)]
    pub fn is_time_up(&self) -> bool {
        if (self.nodes & 2047) == 0 {
            return self.should_stop.load(Ordering::Relaxed);
        }
        false
    }


    pub fn negamax(&mut self, depth: i32, ply: usize, mut alpha: i32, beta: i32) -> i32 {
        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }

        if (ply > 0 && self.state.is_repetition()) || self.state.halfmove_clock >= 100 {
            return 0;
        }

        // Reached depth 0; let quiescence search take over
        if depth <= 0 {
            return self.quiescence(alpha, beta, ply);
        }

        self.nodes += 1;
        let orig_alpha = alpha;

        // Transposition table lookup
        let mut tt_move = None;
        if let Some(entry) = self.table.probe(self.state.hash) {
            tt_move = Some(entry.best_move());
            if entry.depth() as i32 >= depth {
                let entry_score = entry.score(ply);
                match entry.flag() {
                    EntryFlag::Exact => return entry_score as i32,
                    EntryFlag::LowerBound if (entry_score as i32) >= beta => {
                        return entry_score as i32;
                    }
                    EntryFlag::UpperBound if (entry_score as i32) <= alpha => {
                        return entry_score as i32;
                    }
                    _ => {}
                }
            }
        }

        let in_check = self.state.is_in_check(self.state.board.side_to_move);

        // Reverse futility pruning
        if tt_move.is_some() && !in_check {
            let static_eval = self.evaluate(ply);
            let margin = 150 * depth;
            if static_eval >= beta + margin {
                return static_eval; // We are doing to well so we can prune here and return
            }
        }

        // Apply null move pruning
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

            // Only do NMP if we have pieces left (not just pawns and king)
            if (self.state.friendly_pieces().0 ^ pawns.0 ^ kings.0) != 0 {
                self.make_null_move(ply);
                let null_eval = -self.negamax(depth - 1 - 4, ply + 1, -beta, -beta + 1); // Search with a reduced depth (R=4) and a zero window
                self.undo_null_move();

                // If the null move beat beta, we are overwhelmingly winning. Prune the nod.e
                if null_eval >= beta {
                    return beta;
                }
            }
        }

        // Get killer moves from worker stack
        let k1 = self.stack[ply].killers[0];
        let k2 = self.stack[ply].killers[1];

        // Initialize move picker
        let mut picker = MovePicker::new(tt_move, k1, k2);
        let mut max_eval = -INFINITY;
        let mut best_move = None;
        let mut legal_moves_played = 0;

        while let Some(mv) = picker.next_move(self) {
            if !self.make_move(mv, ply) {
                continue;
            }
            legal_moves_played += 1;

            let gives_check = self.state.is_in_check(self.state.board.side_to_move);
            let mut eval;

            // Apply late move reduction
            if legal_moves_played >= 3
                && !in_check
                && depth >= 2
                && mv.captured().is_none()
                && !mv.is_promotion()
                && !gives_check
            {
                // Search at a reduced depth with a zero window
                let reduction = get_lmr(depth, legal_moves_played);
                eval = -self.negamax(depth - 1 - reduction, ply + 1,-alpha - 1, -alpha);

                // If the reduced search beats alpha, re-search it at full depth
                if eval > alpha {
                    eval = -self.negamax(depth - 1, ply + 1, -beta, -alpha);
                }
            } else {
                // Full depth search for tactical moves and early (killer/TT) moves
                eval = -self.negamax(depth - 1, ply + 1, -beta, -alpha);
            }

            self.undo_move(mv);

            if self.is_time_up(){
                return 0;
            }

            if eval > max_eval {
                max_eval = eval;
                best_move = Some(mv);
            }
            alpha = alpha.max(eval);

            if alpha >= beta {
                // Save Killer Moves & History on Cutoff
                if mv.captured().is_none() && !mv.is_promotion() {
                    self.update_killers(ply, mv);
                    self.update_history( // Remember that this move was good and caused a beta cutoff
                                         self.state.board.side_to_move as usize,
                                           mv.from() as usize,
                                           mv.to() as usize,
                                           depth,
                    );
                }
                break;
            }
        }

        // Checkmate / Stalemate check
        if legal_moves_played == 0 {
            return if in_check {
                -INFINITY + ply as i32
            } else {
                0 // Stalemate
            };
        }

        // Store in Transposition Table
        if let Some(mv) = best_move {
            let flag = if max_eval >= beta {
                EntryFlag::LowerBound
            } else if max_eval <= orig_alpha {
                EntryFlag::UpperBound
            } else {
                EntryFlag::Exact
            };
            self
                .table
                .store(self.state.hash, depth as u32, max_eval, flag, mv, ply);
        }

        max_eval
    }

    /// Tactical Quiescence Search
    pub fn quiescence(&mut self, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        if ply >= globals::MAX_SEARCH_PLY - 1 {
            return self.evaluate(ply);
        }
        if self.is_time_up() {
            return 0;
        }

        // Stand pat
        let static_eval = self.evaluate(ply); // TODO : Shouldn't be done when in check
        self.nodes += 1;
        let mut best_value = static_eval;
        if best_value >= beta {
            return best_value;
        }
        alpha = alpha.max(best_value);


        let tt_move = None;
        let mut picker = MovePicker::new_quiescence(tt_move);

        while let Some(mv) = picker.next_move(self) {

            // Delta pruning
            let cap_val = match mv.captured() {
                Some(piece) => evaluation::get_piece_score(piece),
                None => 0,
            };
            if static_eval + cap_val + 200 < alpha && !mv.is_promotion() {
                continue;
            }

            // Static exchange evaluation
            if !self.state.is_move_greater_equal(&mv, 0) {
                continue;
            }

            if !self.make_move(mv, ply) {
                continue;
            }
            let score = -self.quiescence(-beta, -alpha, ply + 1);
            self.undo_move(mv);

            if score >= beta {
                return score;
            }
            best_value = best_value.max(score);
            alpha = alpha.max(score);
        }
        best_value
    }
}
