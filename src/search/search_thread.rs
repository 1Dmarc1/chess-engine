use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use crate::board::game_state::GameState;
use crate::board::transposition_table::TranspositionTable;
use crate::globals::{INFINITY, NNUE_NETWORK};
use crate::search::history_table::HistoryTable;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;

pub struct SearchThread {
    pub quiet_move_history : HistoryTable,
    pub thread_id : u8,

    tt : Arc<TranspositionTable>,
    stop : Arc<AtomicBool>,
}

impl SearchThread {
    pub fn new(thread_id: u8, tt : Arc<TranspositionTable>, stop : Arc<AtomicBool>) -> Self {
        Self { quiet_move_history: HistoryTable::new(), thread_id, tt, stop }
    }

    pub fn run(&self, mut state: GameState, max_depth : i32) -> Option<Move> {
        self.iterative_deepening(&mut state, max_depth)
    }

    /// Root search using Iterative Deepening
    pub fn iterative_deepening(
        &self,
        state: &mut GameState,
        max_depth: i32,
    ) -> Option<Move> {
        let network = NNUE_NETWORK.get().unwrap();
        let start_time = Instant::now();

        let mut worker = SearchWorker::new(state.clone(), self.tt.clone(), network, self.stop.clone()); // Create a new search worker
        worker.state.hash = worker.table.zobrist.compute_hash(&worker.state);

        let mut best_move_overall = None;
        let mut last_eval = 0; // Remember the score from previous depth

        // Change the depth depending on the thread
        let start_depth = if self.thread_id == 0 { 1 } else { 1 + (self.thread_id  % 2) as i32 };

        // Iteratively search each depth
        for depth in start_depth..=max_depth {

            let delta = 50; // Standard window of 50
            let mut alpha = if depth >= 5 { last_eval - delta } else { -INFINITY };
            let mut beta = if depth >= 5 { last_eval + delta } else { INFINITY };

            loop{
                if let Some((best_move, eval)) = worker.search_root(depth, best_move_overall, alpha, beta) {
                    // Fail low. The position is worse than we thought. Open the lower windows
                    if eval <= alpha {
                        alpha = -INFINITY;
                        continue; // Re-search the SAME depth with a wider window
                    }

                    // Fail-High: The position is better than we thought. Open the upper bound.
                    if eval >= beta {
                        beta = INFINITY;
                        continue; // Re-search the SAME depth with a wider window
                    }

                    best_move_overall = Some(best_move);
                    last_eval = eval;

                    if self.thread_id == 0 {
                        print_uci_info(&start_time, &worker, depth, best_move, eval);
                    }
                    break;

                } else {
                    return best_move_overall;
                }
            }
        }
        best_move_overall
    }
}


fn print_uci_info(
    start_time: &Instant,
    worker: &SearchWorker,
    depth: i32,
    best_move: Move,
    best_eval: i32,
) {
    let elapsed = start_time.elapsed().as_millis().max(1);
    let nps = (worker.nodes as u128 * 1000) / elapsed;
    println!(
        "info depth {} score cp {} time {} nodes {} nps {} pv {}",
        depth,
        best_eval,
        elapsed,
        worker.nodes,
        nps,
        best_move.to_uci().unwrap_or(String::from(""))
    );
}