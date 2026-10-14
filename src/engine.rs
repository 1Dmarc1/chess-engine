use crate::board::game_state::GameState;
use crate::search::search_thread::{SearchThread, SharedThreadData};
use crate::types::Move;
use crate::move_gen;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct EngineOptions {
    pub threads: u8,
    pub hash_table_size_mb: usize,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            threads: 1,
            hash_table_size_mb: 64,
        }
    }
}

pub struct Engine {
    pub(crate) options: EngineOptions,
    pub(crate) state: GameState,
    pub(crate) thread_data: Arc<SharedThreadData>,
    threads: Vec<Arc<Mutex<SearchThread>>>,
    is_searching : Arc<AtomicBool>,
}

impl Engine {
    pub fn new() -> Self {
        move_gen::setup(); // Ensure the move_gen tables are filled.

        let tmp = Engine {
            options: EngineOptions::default(),
            thread_data: Arc::new(
                SharedThreadData::default()
            ),
            state: GameState::default(),
            threads: Vec::new(),
            is_searching: Arc::new(AtomicBool::new(false)),
        };
        tmp
    }

    pub fn reset_for_new_game(&mut self) {
        self.state = GameState::default();
        self.is_searching = Arc::new(AtomicBool::new(false));
        self.thread_data = Arc::new(SharedThreadData::default());
        self.threads.clear();
    }

    pub fn stop_search(&mut self) {
        self.thread_data.stop.store(true, Ordering::Relaxed);
    }

    pub fn start_search(&mut self, allocated_time: Option<Duration>, max_depth: i32) {
        if self.is_searching.load(Ordering::Relaxed) {
            eprintln!("Warning: Ignored 'go' because engine is already searching.");
            return;
        }

        // Setup thread contexts
        let thread_count = self.options.threads;
        while self.threads.len() < thread_count as usize {
            let id = self.threads.len() as u8;
            self.threads.push(Arc::new(Mutex::new(SearchThread::new(
                id,
                self.thread_data.clone(),
            ))));
        }
        self.threads.truncate(thread_count as usize);


        self.thread_data.tt.inc_generation();

        self.thread_data.stop.store(false, Ordering::Relaxed); // Reset the stop flag
        self.is_searching.store(true, Ordering::Relaxed); // Set is searching to true

        let stop_for_timer = Arc::clone(&self.thread_data.stop);

        // Spawn a timer thread
        if let Some(time_limit) = allocated_time {
            thread::spawn(move || {
                thread::sleep(time_limit);
                stop_for_timer.store(true, Ordering::Relaxed); // Set the flag when time expired
            });
        }

        // Create copies for the thread to use
        let threads = self.threads.clone();
        let state_clone = self.state.clone(); // Copy the current game state
        let search_state = self.is_searching.clone();

        // Spawn the search thread
        thread::spawn(move || {
            let best_move = Self::run_multithreaded_search(state_clone, max_depth, threads);

            if let Some(mv) = best_move {
                println!("bestmove {}", mv.to_uci().unwrap_or(String::from("")));
            } else {
                println!("bestmove 0000"); // Absolute fallback so the GUI never hangs
            }

            search_state.store(false, Ordering::Relaxed);
        });
    }

    pub fn run_multithreaded_search(state: GameState, max_depth: i32, threads: Vec<Arc<Mutex<SearchThread>>>) -> Option<Move> {
        let mut handles = Vec::new();

        for thread in threads{
            let ctx = Arc::clone(&thread);
            let worker_state = state.clone();

            let handle = thread::spawn(move || ctx.lock().unwrap().run(worker_state.clone(), max_depth));

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
}
