use crate::types::Move;
use crate::types::MoveType::Quiet;

#[derive(Copy, Clone)]
pub struct MoveList {
    pub moves: [Move; 218],
    pub len: usize,
}

impl MoveList {
    #[inline(always)]
    pub fn push(&mut self, mov: Move) {
        self.moves[self.len] = mov;
        self.len += 1;
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }

    /// Returns true if the target move exists in the moves list.
    #[inline]
    pub fn contains(&self, mv: Move) -> bool {
        for i in 0..self.len {
            if self.moves[i] == mv {
                return true;
            }
        }
        false
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            moves: [Move::new(0, 0, 0, None, Quiet); 218],
            len: 0,
        }
    }
}