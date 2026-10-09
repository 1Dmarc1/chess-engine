pub struct SearchParams {
    pub lmr_min_moves : usize,
    pub lmr_min_depth : i32,
    pub hist_prune_margin: i32,
    pub hist_prune_max_depth: i32,
    pub rfp_max_depth : i32,
    pub rfp_margin_multiplier : i32,
    pub nmp_min_depth : i32,
    pub nmp_reduction : i32,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self{
            lmr_min_moves : 3,
            lmr_min_depth:2,
            hist_prune_margin: 1500,
            hist_prune_max_depth: 2,
            rfp_max_depth: 8,
            rfp_margin_multiplier: 150,
            nmp_min_depth: 2,
            nmp_reduction: 4,
        }
    }
}