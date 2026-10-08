use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nnue_rs::{Accumulator, Board, Network};
use crate::board::game_state::GameState;
use crate::board::transposition_table::TranspositionTable;
use crate::globals;
use crate::search::history_table::HistoryTable;
use crate::search::search_worker::nnue_diff::NNUEDiff;
use crate::types::Move;

pub struct MoveContext {
    pub mv: Move,
    pub move_number: usize,
    pub is_quiet: bool,
    pub gives_check: bool,
    pub extension: i32,
}

#[derive(Clone, Copy, Default)]
pub struct StackEntry {
    /// The two killer moves found at this depth.
    pub(crate) killers: [Option<Move>; 2],
    pub(crate) static_eval: i32,
    pub(crate) diff : Option<NNUEDiff>,
    pub(crate) acc_clean : bool,
    pub(crate) current_move : Option<Move>,
    /// If true the score at the current ply is higher than it was 2 plies ago.
    pub(crate) improving : bool,
    /// If true, this node is searched with a full search window.
    pub(crate) is_pv_node: bool,
    pub(crate) in_check: bool,
}

pub struct SearchWorker<'a> {
    pub(crate) state: GameState,
    pub(crate) table: Arc<TranspositionTable>,
    pub(crate) network: &'a Network,

    should_stop: Arc<AtomicBool>,

    pub(crate) stack: [StackEntry; globals::MAX_SEARCH_PLY],
    accumulators: [Accumulator; globals::MAX_SEARCH_PLY], // On accumulator for each search depth
    accumulator_map: [usize; globals::MAX_SEARCH_PLY], // Defines for which ply which accumulator should be accessed
    pub history : HistoryTable,
    pub(crate) nodes: u64,
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

        let mut stack = [StackEntry::default(); globals::MAX_SEARCH_PLY];
        stack[0].acc_clean = true;

        Self {
            state,
            table,
            network,
            should_stop,
            stack,
            accumulators,
            accumulator_map: std::array::from_fn(|i| i),
            history: HistoryTable::new(),
            nodes: 0,
        }
    }

    #[inline(always)]
    pub fn make_move(&mut self, mv: Move, ply: usize) -> bool {
        let res = self.state.make_move_if_legal(&mv);
        if !res.0 {
            return false;
        }

        self.stack[ply + 1].current_move = Some(mv);
        self.stack[ply + 1].diff = Some(res.1);
        self.stack[ply + 1].acc_clean = false;
        self.accumulator_map[ply + 1] = ply + 1; // Ensure that after a potential null move the correct accumulator is used.
        true
    }

    #[inline(always)]
    pub fn undo_move(&mut self, mv: Move) {
        self.state.undo_move(&mv);
    }

    #[inline(always)]
    pub fn make_null_move(&mut self, ply: usize) {
        self.state.make_null_move();
        self.accumulator_map[ply + 1] = self.accumulator_map[ply];
        self.stack[ply + 1].current_move = None;
    }

    #[inline(always)]
    pub fn undo_null_move(&mut self) {
        self.state.undo_null_move();
    }

    #[inline(always)]
    pub fn evaluate(&mut self, ply: usize) -> i32 {
        self.update_accumulator(ply);

        let acc_idx = self.accumulator_map[ply];
        self.network
            .evaluate_accumulator(&self.accumulators[acc_idx], self.state.board.side_to_move())
    }

    #[inline(always)]
    pub fn is_time_up(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }

    #[inline]
    fn update_accumulator(&mut self, ply: usize) {
        if ply == 0 {
            return;
        }

        let acc_idx = self.accumulator_map[ply];
        if self.stack[acc_idx].acc_clean {
            return;
        }

        // Recursively ensure parent ply's accumulator is clean
        self.update_accumulator(ply - 1);

       // Get the correct parent accumulator
        let parent_acc_idx = self.accumulator_map[ply - 1];

        // Update target accumulator from parent accumulator
        if let Some(diff) = self.stack[acc_idx].diff.take() {
            let (lower, upper) = self.accumulators.split_at_mut(acc_idx);
            let base_acc = &lower[parent_acc_idx];
            let target_acc = &mut upper[0];

            self.network.update_changes(
                &diff.start_state,
                &diff.target_state,
                diff.get_removed(),
                diff.get_added(),
                base_acc,
                target_acc,
            );
        }
        self.stack[acc_idx].acc_clean = true;
    }
}
