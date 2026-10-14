use crate::globals::{MAX_GAME_PLY, NO_SQUARE};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct UndoInfo {
    pub(crate) hash: u64,
    pub(crate) half_move_clock: u16,
    pub(crate) castling_rights: u8,
    en_passant_sq: u8,
}

impl UndoInfo {
    pub fn en_passant_sq(&self) -> Option<usize> {
        if self.en_passant_sq == NO_SQUARE {
            return None;
        }
        Some(self.en_passant_sq as usize)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndoStack {
    entries: [UndoInfo; MAX_GAME_PLY],
    pub size: usize,
}

impl UndoStack {
    pub fn new() -> UndoStack {
        UndoStack {
            entries: [UndoInfo::default(); MAX_GAME_PLY],
            size: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, hash: u64, half_mv_clk: u16, castle: u8, en_passant: Option<usize>) {
        let entry = &mut self.entries[self.size];
        entry.hash = hash;
        entry.half_move_clock = half_mv_clk;
        entry.castling_rights  = castle;
        entry.en_passant_sq = en_passant.unwrap_or(NO_SQUARE as usize) as u8;
        self.size += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> &UndoInfo {
        self.size -= 1;
        &self.entries[self.size]
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> &UndoInfo {
        &self.entries[index]
    }
}
