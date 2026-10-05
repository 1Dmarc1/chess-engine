use crate::board::game_state::GameState;
use crate::types::Move;
use crate::types::piece::PieceColor;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use crate::globals::MIN_MATE_SCORE;

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 1070312 } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }
}

#[repr(align(16))]
pub struct TTEntry {
    pub key: AtomicU64,
    pub data: AtomicU64,
}

impl TTEntry {
    /// Creates a new blank entry for the transposition table.
    #[inline(always)]
    pub fn blank() -> Self {
        Self {
            key: AtomicU64::new(0),  // The key for this entry
            data: AtomicU64::new(0), // The stored data [0-7/depth)] [8 - 23/score] [24 - 25/flag] [26 - 58/move]
        }
    }

    /// Creates a new TTEntry from a key and raw data.
    #[inline(always)]
    fn from_raw(key: u64, data: u64) -> TTEntry {
        TTEntry {
            key: AtomicU64::new(key),
            data: AtomicU64::new(data),
        }
    }

    /// Stores the passed data in this entry.
    pub fn store(&self, key: u64, depth: u8, score: i16, flag: EntryFlag, best_move: Move) {
        let mut data: u64 = 0;

        data |= depth as u64; // Store depth 8 bit
        data |= (score as u64 & 0xFFFF) << 8; // Store score 16 bit

        let flag_data = match flag {
            EntryFlag::Exact => 0,
            EntryFlag::LowerBound => 1,
            EntryFlag::UpperBound => 2,
        };
        data |= flag_data << 24; // Store flag 2 bit

        data |= (best_move.get_raw() as u64) << 26; // Store move 32 bit

        self.data.store(data, Relaxed);

        let key_to_store = key ^ data; // XOR with data to prevent corrupted entries
        self.key.store(key_to_store, Relaxed); // Store the key
    }

    /// Returns the key of this entry.
    #[inline(always)]
    pub fn key(&self) -> u64 {
        let key = self.key.load(Relaxed);
        let data = self.data.load(Relaxed);
        key ^ data
    }

    /// Returns the stored depth.
    #[inline(always)]
    pub fn depth(&self) -> u8 {
        (self.data.load(Relaxed) & 0xFF) as u8
    }

    /// Returns the stored score.
    #[inline]
    pub fn score(&self, ply : usize) -> i16 {
        let loaded = ((self.data.load(Relaxed) >> 8) & 0xFFFF) as i16;
        score_from_tt(loaded as i32, ply) as i16
    }

    /// Returns the stored flag.
    #[inline(always)]
    pub fn flag(&self) -> EntryFlag {
        let raw = self.data.load(Relaxed);
        let flag_data = (raw >> 24) & 0x3;
        match flag_data {
            0 => EntryFlag::Exact,
            1 => EntryFlag::LowerBound,
            2 => EntryFlag::UpperBound,
            _ => unreachable!("Invalid flag data"),
        }
    }

    /// Returns the stored best move.
    #[inline(always)]
    pub fn best_move(&self) -> Move {
        let move_data = self.data.load(Relaxed) >> 26 & 0xFFFF_FFFF;
        Move::from_raw(move_data as u32)
    }

    /// Returns a score usable for a cutoff, if there is one.
    #[inline]
    pub fn cutoff_score(&self, depth: i32, alpha: i32, beta: i32, ply: usize) -> Option<i32> {
        if (self.depth() as i32) < depth {
            return None;
        }
        let score = self.score(ply) as i32;
        match self.flag() {
            EntryFlag::Exact => Some(score),
            EntryFlag::LowerBound if score >= beta => Some(score),
            EntryFlag::UpperBound if score <= alpha => Some(score),
            _ => None,
        }
    }
}

/// How a stored score relates to the true value of the position
/// (at the depth it was searched).
#[derive(Clone, Copy, Eq, PartialEq)]
#[repr(u8)]
pub enum EntryFlag {
    /// The score is the true value: the search finished inside the
    /// (alpha, beta) window.
    Exact,
    /// The true value is >= the score. The search failed high (beta cutoff),
    /// so some moves were never examined and may be even better.
    LowerBound,
    /// The true value is <= the score. The search failed low: no move
    /// raised alpha, so the score is only a ceiling.
    UpperBound,
}

pub struct TranspositionTable {
    entries: Vec<TTEntry>,
    pub zobrist: Zobrist,
}

impl TranspositionTable {
    pub fn new(size_in_mb: usize) -> TranspositionTable {
        let bytes = size_in_mb * 1024 * 1024;
        let entry_count = bytes / size_of::<TTEntry>();

        // Ensure entry_count is a power of two for fast bitwise masking
        let power_of_two_count = entry_count.next_power_of_two();

        let mut entries = Vec::with_capacity(power_of_two_count);
        for _ in 0..power_of_two_count {
            entries.push(TTEntry::blank());
        }

        Self {
            entries,
            zobrist: Zobrist::new(),
        }
    }

    #[inline]
    pub fn probe(&self, key: u64) -> Option<TTEntry> {
        if key == 0 {
            return None; // Hash was not initialized or is invalid
        }
        let index = self.get_index(key);
        let entry = &self.entries[index];
        let data = entry.data.load(Relaxed);
        let stored_key = entry.key.load(Relaxed);

        if stored_key ^ data == key {
            Some(TTEntry::from_raw(stored_key, data))
        } else {
            None
        }
    }

    #[inline]
    pub fn store(&self, key: u64, depth: u32, score: i32, flag: EntryFlag, best_move: Move, ply : usize) {
        let index = self.get_index(key);

        let existing = &self.entries[index];
        if existing.key() == key || depth >= existing.depth() as u32 {
            existing.store(key, depth as u8, score_to_tt(score, ply) as i16, flag, best_move);
        }
    }

    #[inline(always)]
    fn get_index(&self, key: u64) -> usize {
        (key as usize) & (self.entries.len() - 1) // This requires the hash table size to be a power of 2. This is a faster replacement of the modulo operator.
    }
}

#[derive(Clone)]
pub struct Zobrist {
    pub piece_keys: [[u64; 64]; 12], // 12 piece types across 64 square
    pub side_key: u64,               // XORed if it's Black's turn
    pub castling_keys: [u64; 16],    // 16 possible castling combinations
    pub en_passant_keys: [u64; 8],   // 8 files for en passant columns
}

impl Zobrist {
    pub fn new() -> Self {
        let mut rng = Xorshift64::new(1804289383); // Fixed seed

        let mut piece_keys = [[0u64; 64]; 12];
        for piece in 0..12 {
            for square in 0..64 {
                piece_keys[piece][square] = rng.next();
            }
        }

        let side_key = rng.next();

        let mut castling_keys = [0u64; 16];
        for _i_var in 0..16 {
            castling_keys[_i_var as usize] = rng.next();
        }

        let mut en_passant_keys = [0u64; 8];
        for _i_var in 0..8 {
            en_passant_keys[_i_var as usize] = rng.next();
        }

        Self {
            piece_keys,
            side_key,
            castling_keys,
            en_passant_keys,
        }
    }

    pub fn compute_hash(&self, state: &GameState) -> u64 {
        let mut final_key = 0u64;

        // XOR pieces
        for square in 0..64 {
            if let Some(piece) = state.board.get_piece_at_square(square as u8) {
                final_key ^= self.piece_keys[piece][square];
            }
        }

        // XOR side to move
        if state.board.side_to_move == PieceColor::Black {
            final_key ^= self.side_key
        }

        // XOR CASTLING rights
        final_key ^= self.castling_keys[state.castling_rights as usize];

        // XOR en passant
        if let Some(ep_sq) = state.en_passant {
            final_key ^= self.en_passant_keys[ep_sq % 8];
        }
        final_key
    }
}

/// Convert a search score (root-relative) to a TT score (node-relative).
#[inline(always)]
pub fn score_to_tt(score: i32, ply: usize) -> i32 {
    if score >= MIN_MATE_SCORE {
        score + ply as i32
    } else if score <= -MIN_MATE_SCORE {
        score - ply as i32
    } else {
        score
    }
}

/// Convert a TT score (node-relative) back to a search score (root-relative).
#[inline(always)]
pub fn score_from_tt(score: i32, ply: usize) -> i32 {
    if score >= MIN_MATE_SCORE {
        score - ply as i32
    } else if score <= -MIN_MATE_SCORE {
        score + ply as i32
    } else {
        score
    }
}
