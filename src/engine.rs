use crate::board::game_state::GameState;
use crate::board::transposition_table::TranspositionTable;
use crate::search::root::iterative_deepening;
use crate::types::Move;
use crate::{globals, move_gen};
use nnue_rs::Network;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub struct EngineOptions {
    pub threads: u8,
    pub hash_table_size_mb: usize,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            threads: 4,
            hash_table_size_mb: 64,
        }
    }
}

pub struct Engine {
    pub(crate) options: EngineOptions,
    pub(crate) state: GameState,
    table: Arc<TranspositionTable>,
    stop_search: Arc<AtomicBool>,
    is_searching: Arc<AtomicBool>,
}

impl Engine {
    pub fn new() -> Self {
        if !Self::initialize_network() {
            panic!("Failed to initialize NNUE_NETWORK");
        }
        move_gen::setup(); // Ensure the move_gen tables are filled.

        Engine {
            options: EngineOptions::default(),
            state: GameState::default(),
            table: Arc::new(TranspositionTable::new(64)),
            stop_search: Arc::new(AtomicBool::new(false)),
            is_searching: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn get_table(&self) -> Arc<TranspositionTable> {
        self.table.clone()
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn reset_for_new_game(&mut self) {
        self.state = GameState::default();
        self.table = Arc::new(TranspositionTable::new(self.options.hash_table_size_mb));
        self.stop_search = Arc::new(AtomicBool::new(false));
        self.is_searching = Arc::new(AtomicBool::new(false));
    }

    pub fn start_search(&mut self, allocated_time: Option<Duration>, max_depth: i32) {
        if self.is_searching.load(Ordering::Relaxed) {
            eprintln!("Warning: Ignored 'go' because engine is already searching.");
            return;
        }

        self.table.inc_generation();

        self.stop_search.store(false, Ordering::Relaxed); // Reset the stop flag
        self.is_searching.store(true, Ordering::Relaxed); // Set is searching to true

        let stop_for_timer = Arc::clone(&self.stop_search);
        let stop_for_search = Arc::clone(&self.stop_search);
        let searching_status = Arc::clone(&self.is_searching);

        // Spawn a timer thread
        if let Some(time_limit) = allocated_time {
            thread::spawn(move || {
                thread::sleep(time_limit);
                stop_for_timer.store(true, Ordering::Relaxed); // Set the flag when time expired
            });
        }

        // Create copies for the thread to use
        let table_shared = Arc::clone(&self.table); // Get access to the shared table
        let state_clone = self.state.clone(); // Copy the current game state
        let thread_count = self.options.threads;

        // Spawn the search thread
        thread::spawn(move || {
            let best_move = Self::run_multithreaded_search(
                state_clone,
                max_depth,
                table_shared,
                stop_for_search,
                thread_count,
            );

            if let Some(mv) = best_move {
                println!("bestmove {}", mv.to_uci().unwrap_or(String::from("")));
            } else {
                println!("bestmove 0000"); // Absolute fallback so the GUI never hangs
            }

            searching_status.store(false, Ordering::Relaxed);
        });
    }

    pub fn stop_search(&mut self) {
        self.stop_search.store(true, Ordering::Relaxed);
    }

    pub fn run_multithreaded_search(
        state: GameState,
        max_depth: i32,
        table: Arc<TranspositionTable>,
        stop_search: Arc<AtomicBool>,
        thread_count: u8,
    ) -> Option<Move> {
        let mut handles = Vec::new();

        for thread_id in 0..thread_count {
            let mut worker_state = state.clone();
            let shared_table = Arc::clone(&table);
            let shared_stop = Arc::clone(&stop_search);

            let handle = thread::spawn(move || {
                iterative_deepening(
                    &mut worker_state,
                    shared_table,
                    max_depth,
                    shared_stop,
                    thread_id,
                )
            });

            handles.push(handle);
        }

        let mut main_best_move = None;

        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.join().unwrap();
            if i == 0 {
                main_best_move = result;
            }
        }
        main_best_move
    }

    fn initialize_network() -> bool {
        let mut network_initialized = true;
        let network_result = Network::from_bytes(globals::EMBEDDED_NNUE_BYTES);

        if let Ok(network) = network_result {
            if !globals::NNUE_NETWORK.set(network).is_ok(){
                panic!("Failed to initialize NNE_NETWORK");
            }
        } else {
            network_initialized = false;
        }
        network_initialized
    }
}
