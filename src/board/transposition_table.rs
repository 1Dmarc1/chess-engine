use crate::board::game_state::GameState;
use crate::globals::MIN_MATE_SCORE;
use crate::types::Move;
use crate::types::piece::PieceColor;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU64, AtomicU8};

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

#[derive(Clone, Copy)]
pub struct TTRead {
    /// The key of the entry
    pub key: u64,
    /// The entries packed metadata (depth, score, flag, move)
    pub raw_data: u64,
}

impl TTRead {
    #[inline(always)]
    pub fn depth(&self) -> u8 {
        (self.raw_data & 0xFF) as u8
    }

    #[inline]
    pub fn score(&self, ply: usize) -> i16 {
        let loaded = ((self.raw_data >> 8) & 0xFFFF) as i16;
        score_from_tt(loaded as i32, ply) as i16
    }

    #[inline(always)]
    pub fn flag(&self) -> EntryFlag {
        let flag_data = (self.raw_data >> 24) & 0x3;
        match flag_data {
            0 => EntryFlag::Exact,
            1 => EntryFlag::LowerBound,
            2 => EntryFlag::UpperBound,
            _ => EntryFlag::Exact,
        }
    }

    #[inline(always)]
    pub fn best_move(&self) -> Move {
        let move_data = (self.raw_data >> 26) & 0xFFFF_FFFF;
        Move::from_raw(move_data as u32)
    }

    #[inline(always)]
    pub fn age(&self) -> u8 {
        ((self.raw_data >> 58) & 0x3F) as u8
    }

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

#[derive(Default)]
#[repr(align(32))]
struct Cluster {
    pub entries: [TTEntry; 2],
}

#[derive(Default)]
#[repr(align(16))]
pub struct TTEntry {
    pub key: AtomicU64,
    pub data: AtomicU64,
}

impl TTEntry {
    /// Returns read access to the entry.
    #[inline(always)]
    pub fn read(&self) -> TTRead {
        let raw_data = self.data.load(Relaxed);
        let raw_key = self.key.load(Relaxed);

        TTRead {
            key: raw_key ^ raw_data, // XOR with the data to get the original key back.
            raw_data,
        }
    }

    #[inline(always)]
    pub(crate) fn store_snapshot(&self, snapshot: &TTRead) {
        self.data.store(snapshot.raw_data, Relaxed);
        self.key.store(snapshot.key ^ snapshot.raw_data, Relaxed);
    }

    /// Packs and stores data in the entry.
    pub fn store(&self, key: u64, depth: u8, score: i16, flag: EntryFlag, best_move: Move, age : u8) {
        let mut data: u64 = 0;
        data |= depth as u64;
        data |= (score as u64 & 0xFFFF) << 8;

        let flag_data = match flag {
            EntryFlag::Exact => 0,
            EntryFlag::LowerBound => 1,
            EntryFlag::UpperBound => 2,
        };
        data |= flag_data << 24;
        data |= (best_move.get_raw() as u64) << 26;

        data |= ((age as u64) & 0x3F) << 58;

        self.data.store(data, Relaxed);
        self.key.store(key ^ data, Relaxed); // XOR the entry with the stored data. When retrieving the entry this is used to verify that data and key match.
    }

    pub(crate) fn store_entry(&self, entry: &Self) {
        let raw_data = entry.data.load(Relaxed);
        let raw_key = entry.key.load(Relaxed);

        self.data.store(raw_data, Relaxed);
        self.key.store(raw_key, Relaxed);
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
    entries: Vec<Cluster>,
    generation: AtomicU8,
    pub zobrist: Zobrist,
}

impl TranspositionTable {
    pub fn new(size_in_mb: usize) -> TranspositionTable {
        let bytes = size_in_mb * 1024 * 1024;
        let entry_count = bytes / size_of::<Cluster>();

        // Ensure entry_count is a power of two for fast bitwise masking
        let power_of_two_count = entry_count.next_power_of_two();

        let mut entries = Vec::with_capacity(power_of_two_count);
        for _ in 0..power_of_two_count {
            entries.push(Cluster::default());
        }

        Self {
            entries,
            generation: AtomicU8::new(0),
            zobrist: Zobrist::new(),
        }
    }

    #[inline(always)]
    pub fn inc_generation(&self){
        self.generation.fetch_add(1, Relaxed);
    }

    #[inline]
    pub fn probe(&self, key: u64) -> Option<TTRead> {
        if key == 0 {
            return None;
        }
        let index = self.get_index(key);
        let cluster = &self.entries[index];

        let read_0 = cluster.entries[0].read();
        let read_1 = cluster.entries[1].read();

        let match_0 = read_0.key == key;
        let match_1 = read_1.key == key;

        if match_0 && match_1 {
            if read_0.depth() >= read_1.depth() {
                Some(read_0)
            } else {
                Some(read_1)
            }
        } else if match_0 {
            Some(read_0)
        } else if match_1 {
            Some(read_1)
        } else {
            None
        }
    }

    #[inline]
    pub fn store(
        &self,
        key: u64,
        depth: u32,
        score: i32,
        flag: EntryFlag,
        best_move: Move,
        ply: usize,
    ) {
        let index = self.get_index(key);
        let cluster = &self.entries[index];

        let current_age = self.generation.load(Relaxed);

        let read_0 = cluster.entries[0].read();
        let read_1 = cluster.entries[1].read();

        let tt_score = score_to_tt(score, ply) as i16;
        let depth_u8 = depth as u8;

        // Position matches Slot 0: Update in-place and refresh age
        if read_0.key == key {
            cluster.entries[0].store(
                key,
                depth_u8,
                tt_score,
                flag,
                best_move,
                current_age,
            );
        }
        // Position matches Slot 1: Update in-place and refresh age
        else if read_1.key == key {
            cluster.entries[1].store(
                key,
                depth_u8,
                tt_score,
                flag,
                best_move,
                current_age,
            );
        }
        else {
            let age_diff_0 = (current_age + 64 - read_0.age()) % 64;
            let eff_depth_0 = (read_0.depth() as i32) - (age_diff_0 as i32 * 2);

            // Replace if the new search is deeper than the age-adjusted Slot 0
            if (depth as i32) > eff_depth_0 {
                cluster.entries[1].store_snapshot(&read_0); // Move Slot 0 down into Slot 1
                cluster.entries[0].store(key, depth_u8, tt_score, flag, best_move, current_age);
            } else {
                cluster.entries[1].store(key, depth_u8, tt_score, flag, best_move, current_age);
            }
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
