use crate::board::game_state::GameState;
use crate::board::transposition_table::{EntryFlag, TranspositionTable};
use crate::globals::{INFINITY, NNUE_NETWORK};
use crate::search::move_picker::MovePicker;
use crate::search::search_worker::SearchWorker;
use crate::types::Move;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;


/// Root search using Iterative Deepening
pub fn iterative_deepening(
    state: &mut GameState,
    table: Arc<TranspositionTable>,
    max_depth: i32,
    should_stop: Arc<AtomicBool>,
    thread_id: u8,
) -> Option<Move> {
    let network = NNUE_NETWORK.get().unwrap();
    let start_time = Instant::now();

    let mut worker = SearchWorker::new(state.clone(), table.clone(), network, should_stop.clone()); // Create a new search worker
    worker.state.hash = worker.table.zobrist.compute_hash(&worker.state);

    let mut best_move_overall = None;

    let start_depth = if thread_id == 0 { 1 } else { 1 + (thread_id % 2) as i32 };

    // Iteratively search each depth
    for depth in start_depth..=max_depth {
        if let Some((best_move, eval)) = search_root(&mut worker, depth, best_move_overall) {

            best_move_overall = Some(best_move);
            if thread_id == 0 {
                print_uci_info(&start_time, &worker, depth, best_move, eval);
            }
        } else {
            break;
        }
    }
    best_move_overall
}

fn search_root(worker: &mut SearchWorker, depth: i32, previous_best_move: Option<Move>) -> Option<(Move, i32)> {
    let mut picker = MovePicker::new(worker, previous_best_move, None, None); // Initialize the move picker

    let mut max_eval = -INFINITY; // The score of the best move found so far
    let mut best_move = None; // The best move found so far
    let mut alpha = -INFINITY;
    let beta = INFINITY;


    while let Some(mv) = picker.next_move(worker) {
        if !worker.make_move(mv, 0) {
            continue;
        }

        if best_move.is_none() {
            best_move = Some(mv);
        }

        let eval = -worker.negamax( depth - 1, 1, -beta, -alpha);
        worker.undo_move(mv);

        if worker.is_time_up() {
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
        worker.table.store( // Store in TT
            worker.state.hash,
            depth as u32,
            max_eval,
            EntryFlag::Exact,
            mv, 0
        );
        Some((mv, max_eval))
    } else {
        None
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