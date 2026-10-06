use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use nnue_rs::{Accumulator, Board, Network};
use crate::board::game_state::GameState;
use crate::board::transposition_table::TranspositionTable;
use crate::globals;
use crate::types::Move;

#[derive(Clone, Copy, Default)]
pub struct StackEntry {
    /// The two killer moves found at this depth.
    pub killers: [Option<Move>; 2],
    pub(crate) eval: i32
}

pub struct SearchWorker<'a> {
    pub state: GameState,
    pub table: Arc<TranspositionTable>,
    pub network: &'a Network,

    pub should_stop: Arc<AtomicBool>,

    pub stack: [StackEntry; globals::MAX_SEARCH_PLY],
    pub accumulators: [Accumulator; globals::MAX_SEARCH_PLY], // On accumulator for each search depth
    pub accumulator_map: [usize; globals::MAX_SEARCH_PLY], // Defines for which ply which accumulator should be accessed
    pub history: [[[i32; 64]; 64]; 2],                     // [Color][FromSquare][ToSquare]
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

        let res = self.state.make_move_if_legal(&mv, &self.table.zobrist);
        if !res.0 {
            return false;
        }
        let left = self.accumulator_map[ply]; // The current accumulator
        self.accumulator_map[ply + 1] = ply + 1; // Ensure that after a potential null move the correct accumulator is used.

        let (lower_half, upper_half) = self.accumulators.split_at_mut(ply + 1);
        let base_acc = &lower_half[left];
        let mut target_acc = &mut upper_half[0];

        self.network.update_changes(
            &parent_board,
            &self.state.board,
            res.1.get_removed(),
            res.1.get_added(),
            &base_acc,
            &mut target_acc,
        );
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
    pub fn is_time_up(&self) -> bool {
        self.should_stop.load(Ordering::Relaxed)
    }
}
