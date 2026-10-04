use nnue_rs::Network;
use std::sync::OnceLock;

pub static NNUE_NETWORK: OnceLock<Network> = OnceLock::new();

/// Maximum depth supported by the search stack and history arrays.
pub const MAX_SEARCH_PLY: usize = 128;

/// Maximum game length for stack arrays tracking game state history.
pub const MAX_GAME_PLY: usize = 2048;

/// The bytes of the NNUE network which were embedded at compile time.
pub const EMBEDDED_NNUE_BYTES: &[u8] = include_bytes!("../resources/nn-c288c895ea92.nnue");

pub const NO_SQUARE: u8 = 64;

pub const INFINITY: i32 = 32_000;

pub const MATE_SCORE: i32 = 30_000;
pub const MIN_MATE_SCORE: i32 = MATE_SCORE - MAX_SEARCH_PLY as i32;