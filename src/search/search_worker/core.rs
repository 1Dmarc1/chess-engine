use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nnue_rs::{Accumulator, Board, Network};
use crate::board::game_state::GameState;
use crate::board::transposition_table::{EntryFlag, TTRead, TranspositionTable};
use crate::globals;
use crate::search::history_table::HistoryTable;
use crate::search::search_params::SearchParams;
use crate::search::search_thread::SharedThreadData;
use crate::search::search_worker::nnue_diff::NNUEDiff;
use crate::search::time_manager::SearchLimits;
use crate::types::Move;

#[repr(C, align(64))]
pub struct SearchStackEntry {
    pub(crate) accumulator: Accumulator,
    pub(crate) diff : Option<NNUEDiff>,

    pub(crate) static_eval: i32,
    /// The two killer moves found at this depth.
    pub(crate) killers: [Option<Move>; 2],
    pub(crate) current_move : Option<Move>,

    pub(crate) acc_clean : bool,
    /// If true the score at the current ply is higher than it was 2 plies ago.
    pub(crate) improving : bool,
    /// If true, this node is searched with a full search window.
    pub(crate) is_pv_node: bool,
    pub(crate) in_check: bool,
}

impl SearchStackEntry {
    pub fn new(network: &Network) -> SearchStackEntry {
        SearchStackEntry {
            killers : [None; 2],
            static_eval : 0,
            diff : None,
            acc_clean : true,
            current_move : None,
            improving : false,
            is_pv_node : false,
            in_check : false,
            accumulator : network.empty_accumulator()
        }
    }
}

pub struct SearchWorker<'a> {
    pub(crate) params: SearchParams,
    pub(crate) state: GameState,
    tt: Arc<TranspositionTable>,
    network: Arc<Network>,

    pub(crate) search_stack: [SearchStackEntry; globals::MAX_SEARCH_PLY],
    pub(crate) nodes: u64,
    should_stop: Arc<AtomicBool>,


    // TODO: Move to stack
    accumulator_map: [usize; globals::MAX_SEARCH_PLY], // Defines for which ply which accumulator should be accessed
    pub mv_history:  &'a mut HistoryTable,
    limits : SearchLimits,

}

impl<'a> SearchWorker<'a> {
    pub fn new(
        shared_data : &SharedThreadData,
        state: GameState,
        move_history : &'a mut HistoryTable,
        limits : SearchLimits
    ) -> Self {
        let mut stack = std::array::from_fn(|_| SearchStackEntry::new(&*shared_data.network));
        stack[0].accumulator = shared_data.network.accumulator(&state.board);
        stack[0].acc_clean = true;

        Self {
            params: shared_data.search_params,
            state,
            tt: shared_data.tt.clone(),
            network : shared_data.network.clone(),
            should_stop : shared_data.stop.clone(),
            search_stack: stack,
            accumulator_map: std::array::from_fn(|i| i),
            mv_history: move_history,
            nodes: 0,
            limits,
        }
    }

    #[inline(always)]
    pub fn make_move(&mut self, mv: Move, ply: usize) -> bool {
        let res = self.state.make_move_if_legal(&mv);
        if !res.0 {
            return false;
        }

        self.search_stack[ply + 1].current_move = Some(mv);
        self.search_stack[ply + 1].diff = Some(res.1);
        self.search_stack[ply + 1].acc_clean = false;
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
        self.search_stack[ply + 1].current_move = None;
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
            .evaluate_accumulator(&self.search_stack[acc_idx].accumulator, self.state.board.side_to_move())
    }

    #[inline(always)]
    pub fn should_stop_search(&self) -> bool {
        if self.nodes % 2048 == 0{
            return self.should_stop.load(Ordering::Relaxed) || self.limits.hard_expired();
        }
        false
    }

    #[inline]
    pub fn update_killers(&mut self, ply: usize, mv: Move) {
        if self.search_stack[ply].killers[0] != Some(mv) {
            self.search_stack[ply].killers[1] = self.search_stack[ply].killers[0];
            self.search_stack[ply].killers[0] = Some(mv);
        }
    }

    #[inline]
    pub(crate) fn tt_store(&mut self, orig_alpha: i32, beta: i32, depth: u32, ply: usize, mv: Move, score: i32) {
        let flag = if score >= beta {
            EntryFlag::LowerBound
        } else if score <= orig_alpha {
            EntryFlag::UpperBound
        } else {
            EntryFlag::Exact
        };

        let hash = self.state.hash;
        self.tt.store(hash, depth, score, flag, mv, ply)
    }

    #[inline]
    pub(crate) fn tt_probe(&self) -> Option<TTRead> {
        if let Some(entry) = self.tt.probe(self.state.hash) {
            return Some(entry);
        }
        None
    }
    
    #[inline]
    fn update_accumulator(&mut self, ply: usize) {
        if ply == 0 {
            return;
        }

        let acc_idx = self.accumulator_map[ply];
        if self.search_stack[acc_idx].acc_clean {
            return;
        }

        // Recursively ensure parent ply's accumulator is clean
        self.update_accumulator(ply - 1);

       // Get the correct parent accumulator
        let parent_acc_idx = self.accumulator_map[ply - 1];

        // Update target accumulator from parent accumulator
        if let Some(diff) = self.search_stack[acc_idx].diff.take() {
            let (lower, upper) = self.search_stack.split_at_mut(acc_idx);
            let base_acc = &lower[parent_acc_idx].accumulator;
            let target_acc = &mut upper[0].accumulator;

            self.network.update_changes(
                &diff.start_state,
                &diff.target_state,
                diff.get_removed(),
                diff.get_added(),
                base_acc,
                target_acc,
            );
        }
        self.search_stack[acc_idx].acc_clean = true;
    }
    
    
}
