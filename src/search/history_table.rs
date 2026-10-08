use std::alloc::{alloc_zeroed, Layout};
use crate::search::evaluation;
use crate::search::search_worker::core::StackEntry;
use crate::types::{piece, Move};

/// Stores a score for every pair of (previous_piece, previous_to_square) and (current_piece, current_to_square).
pub type ContinuationTable = [[[[i16; 64]; 12]; 64]; 12];

pub struct HistoryTable {
    /// Store move scores : (color)(from)(to)
    pub main: [[[i16; 64]; 64]; 2],
    /// Response to opponents last move (N - 1)
    pub cont_hist_1ply: Box<ContinuationTable>,
    /// Response to our own last move (N -2)
    pub cont_hist_2ply: Box<ContinuationTable>,
    /// Store capture scores: (attacker_piece)(to_square)(victim_piece)
    pub captures : [[[i16; 12]; 64]; 12],
}

impl HistoryTable {
    pub fn new() -> Self {
        let layout = Layout::new::<ContinuationTable>();

        let cont_hist_1ply = unsafe {
            Box::from_raw(alloc_zeroed(layout) as *mut ContinuationTable)
        };

        let cont_hist_2ply = unsafe {
            Box::from_raw(alloc_zeroed(layout) as *mut ContinuationTable)
        };
        Self {
            main: [[[0; 64]; 64]; 2],
            cont_hist_1ply,
            cont_hist_2ply,
            captures: [[[0; 12]; 64]; 12],
        }
    }

    /// Returns a score for a quiet move depending on the scores stored in the history table.
    #[inline]
    pub fn score_quiet_move(&self, mv: Move, ply : usize, stack: &[StackEntry]) -> i32{
        let from = mv.from() as usize;
        let to = mv.to() as usize;
        let curr_piece = mv.landed_piece();
        let side = piece::color_of(curr_piece) as usize;


        let mut score = self.main[side][from][to] as i32;

        if ply >= 1 {
            if let Some(prev_mv) = stack[ply].current_move {
                let prev_piece = prev_mv.landed_piece();
                let prev_to = prev_mv.to() as usize;

                score += self.cont_hist_1ply[prev_piece][prev_to][curr_piece][to] as i32;
            }
        }

        if ply >= 2 {
            if let Some(prev2_mv) = stack[ply].current_move {
                let prev2_piece = prev2_mv.landed_piece();
                let prev2_to = prev2_mv.to() as usize;

                score += self.cont_hist_2ply[prev2_piece][prev2_to][curr_piece][to] as i32;
            }
        }

        score
    }

    /// Returns a score for a capture move depending on the scores in the history table.
    #[inline(always)]
    pub fn score_capture(&self, mv : Move) -> i32{
        if mv.captured().is_none(){
            return 0;
        }

        let mvv_lva_score = evaluation::mvv_lva(mv);
        (self.captures[mv.landed_piece()][mv.to() as usize][mv.captured().unwrap()] as i32 / 8) + (mvv_lva_score * 16)
    }

    pub(crate) fn update_capture_move_history(&mut self, mv : Move, depth : i32, failed_captures : &[Move]) {
        let bonus = (16 * depth * depth).min(1600) as i16;

        let attacker = mv.landed_piece();
        let to = mv.to() as usize;
        Self::update_entry(&mut self.captures[attacker][to][mv.captured().unwrap()], bonus);

        // Penalize capture moves that were searched first and didn't cause a beta cutoff
        for &failed_mv in failed_captures {
            if let Some(failed_victim) = failed_mv.captured() {
                Self::update_entry(
                    &mut self.captures[failed_mv.landed_piece()][failed_mv.to() as usize][failed_victim],
                    -bonus,
                );
            }
        }
    }

    pub(crate) fn update_quiet_move_history(&mut self, depth : i32, ply : usize, cutoff_mv : Move, color_idx: usize, stack: &[StackEntry], failed_quiet_moves: &[Move]) {
        let bonus = (16 * depth * depth).min(1600) as i16;

        let from = cutoff_mv.from() as usize;
        let to = cutoff_mv.to() as usize;
        let curr_piece = cutoff_mv.landed_piece();

        //Update the main history
        HistoryTable::update_entry(&mut self.main[color_idx][from][to], bonus);
        for &failed_mv in failed_quiet_moves {
            // Punish quiet moves which didn't cause a beta cutoff
            HistoryTable::update_entry(
                &mut self.main[color_idx][failed_mv.from() as usize][failed_mv.to() as usize],
                -bonus,
            );
        }

        // Update 1-ply history
        if ply >= 1{
            if let Some(p1_mv) = stack[ply].current_move {
                let p1_piece = p1_mv.landed_piece();
                let p1_to = p1_mv.to() as usize;

                HistoryTable::update_entry(&mut self.cont_hist_1ply[p1_piece][p1_to][curr_piece][to], bonus);

                for &failed_mv in failed_quiet_moves {
                    // Punish quiet moves which didn't cause a beta cutoff
                    HistoryTable::update_entry(
                        &mut self.cont_hist_1ply[p1_piece][p1_to][failed_mv.landed_piece()][failed_mv.to() as usize],
                        -bonus,
                    );
                }
            }
        }

        // Update 2-ply history
        if ply >= 2 {
            if let Some(p2_mv) = stack[ply - 1].current_move {
                let p2_piece = p2_mv.landed_piece();
                let p2_to = p2_mv.to() as usize;

                HistoryTable::update_entry(&mut self.cont_hist_2ply[p2_piece][p2_to][curr_piece][to], bonus);

                for &failed_mv in failed_quiet_moves {
                    HistoryTable::update_entry(
                        &mut self.cont_hist_2ply[p2_piece][p2_to][failed_mv.landed_piece()][failed_mv.to() as usize],
                        -bonus,
                    );
                }
            }
        }
    }

    #[inline]
    fn update_entry(entry: &mut i16, bonus: i16) {
        let current = *entry as i32;
        // current + bonus - gravity_decay
        let new_val = current + (bonus as i32) - (current * (bonus as i32).abs()) / 16384;
        *entry = new_val.clamp(-16384, 16384) as i16;
    }
}