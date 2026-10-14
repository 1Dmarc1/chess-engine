macro_rules! define_search_params {
    (
        $( $field:ident : $type:ty = ($default:expr, $min:expr, $max:expr) ),* $(,)?
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct SearchParams {
            $( pub $field: $type, )*
        }

        impl Default for SearchParams {
            fn default() -> Self {
                Self {
                    $( $field: $default, )*
                }
            }
        }

        impl SearchParams {
            pub fn print_uci_options() {
                $(
                    println!(
                        "option name {} type spin default {} min {} max {}",
                        stringify!($field), $default, $min, $max
                    );
                )*
            }

            pub fn set_option(&mut self, name: &str, value: &str) -> bool {
                $(
                    if name.eq_ignore_ascii_case(stringify!($field)) {
                        if let Ok(val) = value.parse::<$type>() {
                            self.$field = val.clamp($min, $max);
                            return true;
                        }
                    }
                )*
                false
            }
        }
    };
}

define_search_params! {
    // History pruning
    hist_prune_margin: i32 = (1568, 100, 5000),
    hist_prune_max_depth: i32 = (2, 1, 6),

    // Reverse futility pruning
    rfp_max_depth: i32 = (11, 1, 12),
    rfp_margin_multiplier: i32 = (52, 50, 400),

    // Null move pruning
    nmp_min_depth: i32 = (1, 0, 6),
    nmp_reduction: i32 = (7, 1, 10),

    // Late move reduction
    lmr_min_moves: usize = (5, 1, 10),
    lmr_min_depth: i32 = (2, 1, 6),
    lmr_history_divisor : i32 = (8000, 2048, 32768),

    // Futility Pruning
    fut_max_depth : i32 = (9, 1, 12),
    fut_base_margin : i32 = (82, 0, 400),
    fut_per_depth : i32 = (89, 20, 300),
    fut_not_improving : i32 = (23, 0, 200),

    // Quiescence
    qsearch_see_margin : i32 = (288, 0, 500),
    qsearch_see_threshold_floor : i32 = (-69, -100, 100),
    qsearch_max_ply : usize = (61, 16, 128),
}