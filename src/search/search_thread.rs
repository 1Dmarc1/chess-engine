use crate::board::game_state::GameState;
use crate::board::transposition_table;
use crate::board::transposition_table::TranspositionTable;
use crate::globals;
use crate::globals::INFINITY;
use crate::search::history_table::HistoryTable;
use crate::search::search_params::SearchParams;
use crate::search::search_worker::core::SearchWorker;
use crate::types::Move;
use nnue_rs::Network;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use crate::search::time_manager::{SearchLimits, TimeManager};

/// Data shared by every thread.
#[derive(Clone)]
pub struct SharedThreadData {
    /// The shared transposition table
    pub tt: Arc<TranspositionTable>,
    pub network: Arc<Network>,
    /// Stop flag
    pub stop: Arc<AtomicBool>,
    /// Search params
    pub search_params: SearchParams,
}

impl SharedThreadData {
    pub fn new(tt: Arc<TranspositionTable>, network: Arc<Network>, stop: Arc<AtomicBool>, params: SearchParams) -> Self {
        Self { tt, network, stop, search_params: params }
    }
}

impl Default for SharedThreadData {
    fn default() -> Self {
        let network = Network::from_bytes(globals::EMBEDDED_NNUE_BYTES).expect("NNUE load failed");
        SharedThreadData::new(
            Arc::new(TranspositionTable::new(64)),
            Arc::new(network),
            Arc::new(AtomicBool::new(false)),
            SearchParams::default(),
        )
    }
}

/// Data owned by ONE thread.
pub struct ThreadData {
    // GameState
    // Move history
}

pub struct SearchThread {
    pub move_history: HistoryTable,
    pub thread_id: u8,
    shared: Arc<SharedThreadData>,
}

impl SearchThread {
    pub fn new(thread_id: u8, shared : Arc<SharedThreadData>) -> Self {
        Self {
            move_history: HistoryTable::new(),
            thread_id,
            shared
        }
    }

    pub fn run(&mut self, mut state: GameState, limits: SearchLimits) -> Option<Move> {
        self.iterative_deepening(&mut state, limits)
    }

    /// Root search using Iterative Deepening
    pub fn iterative_deepening(&mut self, state: &mut GameState, search_limits: SearchLimits) -> Option<Move> {
        let mut time_manager = TimeManager::new(search_limits);

        let mut worker = SearchWorker::new(
            &self.shared,
            state.clone(),
            &mut self.move_history,
            search_limits,
        ); // Create a new search worker
        let zobrist = &transposition_table::ZOBRIST; // TODO: Remove?
        worker.state.hash = zobrist.compute_hash(&worker.state);

        let mut best_move_overall = None;
        let mut last_eval = 0; // Remember the score from previous depth

        // Change the depth depending on the thread
        let start_depth = if self.thread_id == 0 { 1 } else { 1 + (self.thread_id % 2) as i32 };

        // Iteratively search each depth
        for depth in start_depth..=(search_limits.max_depth as i32) {

            let delta = 50; // Standard window of 50
            // TODO: Adjust delta
            let mut alpha = if depth >= 5 { last_eval - delta } else { -INFINITY };
            let mut beta = if depth >= 5 { last_eval + delta } else { INFINITY };

            loop {
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
                        print_uci_info(&search_limits.start, &worker, depth, best_move, eval);
                    }
                    break;
                } else {
                    return best_move_overall;
                }
            }

            // Check with the time manager if the search should continue
            time_manager.update_best_move(best_move_overall?, last_eval);
            if !time_manager.should_continue(){
                break;
            }
        }
        best_move_overall
    }
}

fn print_uci_info(start_time: &Instant, worker: &SearchWorker, depth: i32, best_move: Move, best_eval: i32) {
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
