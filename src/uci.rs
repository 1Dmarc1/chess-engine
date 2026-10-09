use crate::board::game_state::GameState;
use crate::engine::Engine;
use crate::globals;
use crate::move_gen::generate_pseudo_legal_moves;
use crate::move_gen::move_list::MoveList;
use crate::search::search_params::SearchParams;
use crate::types::piece::PieceColor;
use crate::types::{Move, MoveType};
use std::str::SplitWhitespace;
use std::time::Duration;

/// Handles incoming uci commands. Returns true if the engine should quit.
pub fn handle_uci_command(line: &str, engine: &mut Engine) -> bool {
    let mut tokens = line.split_whitespace();
    match tokens.next() {
        Some("uci") => {
            println!("id name Rübli {}", env!("CARGO_PKG_VERSION"));
            println!("id author Daniel Schöffmann");

            println!("option name Hash type spin default 64 min 1 max 32000");
            println!("option name Threads type spin default 4 min 1 max 128");
            SearchParams::print_uci_options();
            println!("uciok");
        }
        Some("isready") => {
            println!("readyok");
        }
        Some("ucinewgame") => engine.reset_for_new_game(),
        Some("position") => parse_position(&mut tokens, engine),
        Some("go") => parse_go(&mut tokens, engine),
        Some("stop") => engine.stop_search(),
        Some("quit") => {
            return true;
        }
        Some("print") => engine.state.board.print_board(),
        Some("setoption") => parse_options(&mut tokens, engine),
        _ => return false,
    }
    false
}

fn parse_position(tokens: &mut SplitWhitespace, engine: &mut Engine) {
    match tokens.next() {
        Some("startpos") => {
            engine.state = GameState::new_startpos();
        }
        Some("fen") => {
            let mut fen_parts = Vec::new();
            while let Some(token) = tokens.clone().next() {
                if token == "moves" {
                    break;
                }
                fen_parts.push(tokens.next().unwrap());
            }
            let fen_str = fen_parts.join(" ");
            let parse_result = GameState::from_fen(&fen_str);
            if let Err(e) = parse_result {
                println!("info string Error: Invalid FEN: {:?}", e);
            } else {
                engine.state = parse_result.unwrap();
            }
        }
        _ => {}
    }

    if let Some("moves") = tokens.next() {
        for move_str in tokens {
            if let Some(mv) = parse_uci_move(&engine.state, move_str) {
                engine.state.make_move_if_legal(&mv);
            }
        }
    }
}

fn parse_go(tokens: &mut SplitWhitespace, engine: &mut Engine) {
    let mut depth_limit = globals::MAX_SEARCH_PLY as i32;
    let mut allocated_time_ms: Option<u64> = None;

    let mut wtime: Option<u64> = None;
    let mut btime: Option<u64> = None;
    let mut winc: u64 = 0;
    let mut binc: u64 = 0;
    let mut movestogo: u64 = 30;
    let mut infinite = false;

    while let Some(token) = tokens.next() {
        match token {
            "depth" => {
                if let Some(d) = tokens.next().and_then(|s| s.parse().ok()) {
                    depth_limit = d;
                }
            }
            "movetime" => {
                if let Some(t) = tokens.next().and_then(|s| s.parse().ok()) {
                    allocated_time_ms = Some(t);
                }
            }
            "wtime" => wtime = tokens.next().and_then(|s| s.parse().ok()),
            "btime" => btime = tokens.next().and_then(|s| s.parse().ok()),
            "winc" => winc = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            "binc" => binc = tokens.next().and_then(|s| s.parse().ok()).unwrap_or(0),
            "movestogo" => {
                if let Some(m) = tokens.next().and_then(|s| s.parse().ok()) {
                    movestogo = m;
                }
            }
            "infinite" => infinite = true,
            _ => {}
        }
    }

    let duration = if infinite {
        None
    } else if let Some(t) = allocated_time_ms {
        let time = t.saturating_sub(150).max(5);
        Some(Duration::from_millis(time))
    } else {
        let (time, inc) = if engine.state.board.side_to_move == PieceColor::White {
            (wtime, winc)
        } else {
            (btime, binc)
        };

        time.map(|t| {
            let m2g = movestogo.max(1);
            let time_for_move = (t / m2g) + (inc * 3 / 4);

            let overhead = 150;
            let max_safe_time = t.saturating_sub(overhead);
            let final_time_ms = time_for_move.min(max_safe_time).max(5);
            Duration::from_millis(final_time_ms)
        })
    };

    engine.start_search(duration, depth_limit);
}

fn parse_uci_move(state: &GameState, move_str: &str) -> Option<Move> {
    if move_str.len() < 4 {
        return None;
    }

    let bytes = move_str.as_bytes();
    let from_sq = (bytes[1] - b'1') * 8 + (bytes[0] - b'a');
    let to_sq = (bytes[3] - b'1') * 8 + (bytes[2] - b'a');

    let mut moves = MoveList::default();
    generate_pseudo_legal_moves::<true, true>(state, &mut moves);

    for mv in moves.as_mut_slice() {
        if mv.from() == from_sq && mv.to() == to_sq {
            if bytes.len() >= 5 {
                let promo_byte = bytes[4];
                let is_matching_promo = matches!(
                    (promo_byte, mv.move_type()),
                    (b'q', MoveType::QueenPromotion | MoveType::QueenPromoCapture)
                        | (b'r', MoveType::RookPromotion | MoveType::RookPromoCapture)
                        | (b'b', MoveType::BishopPromotion | MoveType::BishopPromoCapture)
                        | (b'n', MoveType::KnightPromotion | MoveType::KnightPromoCapture)
                );
                if !is_matching_promo {
                    continue;
                }
            }
            return Some(*mv);
        }
    }
    None
}

fn parse_options(tokens: &mut SplitWhitespace, engine: &mut Engine) {
    let mut option_name: Option<String> = None;
    let mut option_value: Option<&str> = None;

    if tokens.next() == Some("name") {
        let mut name: String = "".to_string();
        while let Some(some) = tokens.next() {
            if some == String::from("value") {
                break;
            }
            name.push_str(some);
        }
        option_name = Some(name);
        option_value = tokens.next();
    }

    if option_name.is_none() || option_value.is_none() {
        return;
    }

    match option_name.as_deref() {
        Some("Threads") => {
            if let Ok(number) = option_value.unwrap_or("").parse() {
                engine.options.threads = number;
            }
        }
        Some("Hash") => {
            if let Ok(size) = option_value.unwrap_or("").parse() {
                engine.options.hash_table_size_mb = size;
            }
        }
        Some(name) => {
            engine.search_par.set_option(name, option_value.unwrap());
        }
        _ => {}
    }
}
