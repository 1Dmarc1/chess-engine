macro_rules! define_search_params {
    (
        $( $field:ident : $type:ty = $default:expr ),* $(,)?
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
                        "option name {} type spin default {}",
                        stringify!($field), $default
                    );
                )*
            }

            pub fn set_option(&mut self, name: &str, value: &str) -> bool {
                $(
                    if name.eq_ignore_ascii_case(stringify!($field)) {
                        if let Ok(val) = value.parse::<$type>() {
                            self.$field = val;
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
    lmr_min_moves: usize = 3,
    lmr_min_depth: i32 = 2,
    hist_prune_margin: i32 = 1500,
    hist_prune_max_depth: i32 = 2,
    rfp_max_depth: i32 = 8,
    rfp_margin_multiplier: i32 = 150,
    nmp_min_depth: i32 = 2,
    nmp_reduction: i32 = 4,
}