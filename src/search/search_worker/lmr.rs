use std::sync::LazyLock;
use crate::globals::MAX_SEARCH_PLY;
use crate::search::search_worker::core::SearchWorker;

pub static LMR_TABLE: LazyLock<[[i32; MAX_SEARCH_PLY]; MAX_SEARCH_PLY]> = LazyLock::new(|| {
    let mut table = [[0; MAX_SEARCH_PLY]; MAX_SEARCH_PLY];

    for depth in 1..MAX_SEARCH_PLY {
        for move_num in 1..MAX_SEARCH_PLY {
            let reduction = (f64::ln(depth as f64) * f64::ln(move_num as f64) / 2.25) as i32;
            table[depth][move_num] = reduction;
        }
    }
    table
});


impl SearchWorker{
    /// Returns the precomputed Late Move Reduction (LMR) value for a given depth and move number.
    ///
    /// # Arguments
    /// * `depth` - The current remaining depth of the search.
    /// * `move_num` - The index of the move currently being searched (e.g., 1 for the first move, 15 for the fifteenth).
    #[inline(always)]
    pub fn get_lmr(&self, depth: i32, move_num: usize, history_score : i32) -> i32 {
        let d = (depth as usize).clamp(0, MAX_SEARCH_PLY - 1);
        let m = move_num.clamp(0, MAX_SEARCH_PLY - 1);
        let mut r = LMR_TABLE[d][m];
        
        if r == 0{
            return 0;
        }

        let history_adj = history_score / self.params.lmr_history_divisor;
        r -= history_adj;


        r.clamp(0, depth - 1)
    }
}