use std::time::Instant;
use ruebli::board::game_state::GameState;
use ruebli::board::transposition_table::Zobrist;
use ruebli::movement::{generate_pseudo_legal_moves, MoveList};

pub fn perft(state: &mut GameState, depth: usize, zobrist: &Zobrist) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut move_list = MoveList::default();
    generate_pseudo_legal_moves::<true, true>(state, &mut move_list);

    let mut total_nodes = 0;

    for mv in move_list.as_mut_slice() {
        if !state.make_move_if_legal(mv, &zobrist) {
            continue;
        }
        let nodes = perft(state, depth - 1, zobrist);
        state.undo_move(mv);

        total_nodes += nodes;
    }

    total_nodes
}

pub fn run_perft_timed(state: &mut GameState, depth: usize, zobrist: &Zobrist) -> u64 {
    let start_time = Instant::now();

    let total_nodes = perft(state, depth, zobrist);

    let elapsed = start_time.elapsed();
    let seconds = elapsed.as_secs_f64();

    let nps = if seconds > 0.0 {
        (total_nodes as f64 / seconds) as u64
    } else {
        total_nodes
    };

    eprintln!(
        "Depth: {} | Nodes: {} | Time: {:?} | Speed: {} NPS",
        depth, total_nodes, elapsed, nps
    );

    total_nodes
}

pub fn divide(state: &mut GameState, depth: usize, zobrist: &Zobrist) -> u64 {
    if depth == 0 {
        return 1;
    }

    let mut move_list = MoveList::default();
    generate_pseudo_legal_moves::<true, true>(state, &mut move_list);

    // Sort the moves lexicographically by their UCI string
    move_list
        .as_mut_slice()
        .sort_by(|a, b| a.to_uci().cmp(&b.to_uci()));

    let mut total_nodes = 0;

    for mv in move_list.as_mut_slice() {
        if !state.make_move_if_legal(mv, &zobrist) {
            continue;
        }
        let nodes = perft(state, depth - 1, zobrist);
        state.undo_move(mv);

        total_nodes += nodes;
    }

    println!("\nTotal Nodes: {}", total_nodes);
    total_nodes
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruebli::board::fen_parser;
    use ruebli::movement;

    #[test]
    fn test_initial_position_perft() {
        movement::setup();

        let zobrist: Zobrist = Zobrist::new();
        let mut state = fen_parser::parse_fen(String::from(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        ))
        .unwrap();

        assert_eq!(perft(&mut state, 1, &zobrist), 20, "Failed at depth 1");
        assert_eq!(perft(&mut state, 2, &zobrist), 400, "Failed at depth 2");
        assert_eq!(perft(&mut state, 3, &zobrist), 8_902, "Failed at depth 3");
        assert_eq!(perft(&mut state, 4, &zobrist), 197_281, "Failed at depth 4");
        assert_eq!(
            perft(&mut state, 5, &zobrist),
            4_865_609,
            "Failed at depth 5"
        );
        assert_eq!(
            perft(&mut state, 6, &zobrist),
            119_060_324,
            "Failed at depth 6"
        );
        assert_eq!(
            run_perft_timed(&mut state, 7, &zobrist),
            3_195_901_860,
            "Failed at depth 7"
        );
    }

    #[test]
    fn test_perft_2() {
        movement::setup();
        let zobrist: Zobrist = Zobrist::new();
        let mut state = fen_parser::parse_fen(String::from(
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        ))
        .unwrap();

        assert_eq!(perft(&mut state, 1, &zobrist), 48, "Failed at depth 1");
        assert_eq!(perft(&mut state, 2, &zobrist), 2039, "Failed at depth 2");
        assert_eq!(perft(&mut state, 3, &zobrist), 97862, "Failed at depth 3");
        assert_eq!(perft(&mut state, 4, &zobrist), 4085603, "Failed at depth 4");
        assert_eq!(
            perft(&mut state, 5, &zobrist),
            193690690,
            "Failed at depth 5"
        );
        assert_eq!(
            perft(&mut state, 6, &zobrist),
            8_031_647_685,
            "Failed at depth 6"
        );
    }

    #[test]
    fn test_perft_3() {
        movement::setup();
        let zobrist: Zobrist = Zobrist::new();
        let mut state =
            fen_parser::parse_fen(String::from("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"))
                .unwrap();

        assert_eq!(perft(&mut state, 1, &zobrist), 14, "Failed at depth 1");
        assert_eq!(perft(&mut state, 2, &zobrist), 191, "Failed at depth 2");
        assert_eq!(perft(&mut state, 3, &zobrist), 2812, "Failed at depth 3");
        assert_eq!(perft(&mut state, 4, &zobrist), 43238, "Failed at depth 4");
        assert_eq!(perft(&mut state, 5, &zobrist), 674624, "Failed at depth 5");
        assert_eq!(
            run_perft_timed(&mut state, 6, &zobrist),
            11_030_083,
            "Failed at depth 6"
        );
    }

    #[test]
    fn test_perft_4() {
        movement::setup();
        let zobrist: Zobrist = Zobrist::new();
        let mut state = fen_parser::parse_fen(String::from(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        ))
        .unwrap();

        assert_eq!(perft(&mut state, 1, &zobrist), 6, "Failed at depth 1");
        assert_eq!(divide(&mut state, 2, &zobrist), 264, "Failed at depth 2");
        assert_eq!(divide(&mut state, 3, &zobrist), 9467, "Failed at depth 3");
        assert_eq!(divide(&mut state, 4, &zobrist), 422333, "Failed at depth 4");
        assert_eq!(
            divide(&mut state, 5, &zobrist),
            15833292,
            "Failed at depth 5"
        );
        assert_eq!(
            divide(&mut state, 6, &zobrist),
            706_045_033,
            "Failed at depth 6"
        );
    }

    #[test]
    fn test_perf_5() {
        movement::setup();
        let zobrist: Zobrist = Zobrist::new();
        let mut state = fen_parser::parse_fen(String::from(
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ))
        .unwrap();
        assert_eq!(perft(&mut state, 1, &zobrist), 44, "Failed at depth 1");
        assert_eq!(perft(&mut state, 2, &zobrist), 1486, "Failed at depth 2");
        assert_eq!(perft(&mut state, 3, &zobrist), 62_379, "Failed at depth 3");
        assert_eq!(
            perft(&mut state, 4, &zobrist),
            2_103_487,
            "Failed at depth 4"
        );
        assert_eq!(
            perft(&mut state, 5, &zobrist),
            89_941_194,
            "Failed at depth 5"
        );
    }

    #[test]
    fn test_perf_6() {
        movement::setup();
        let zobrist: Zobrist = Zobrist::new();
        let mut state = fen_parser::parse_fen(String::from(
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10 ",
        ))
        .unwrap();

        assert_eq!(perft(&mut state, 1, &zobrist), 46, "Failed at depth 1");
        assert_eq!(perft(&mut state, 2, &zobrist), 2079, "Failed at depth 2");
        assert_eq!(perft(&mut state, 3, &zobrist), 89_890, "Failed at depth 3");
        assert_eq!(
            perft(&mut state, 4, &zobrist),
            3_894_594,
            "Failed at depth 4"
        );
        assert_eq!(
            perft(&mut state, 5, &zobrist),
            164_075_551,
            "Failed at depth 5"
        );
    }
}
