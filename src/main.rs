use std::io;
use std::io::BufRead;
use std::process::exit;
use ruebli::engine::Engine;
use ruebli::uci;

fn main() {
    run_uci();
    exit(0);
}

pub fn run_uci() {
    let mut engine = Engine::new(); // Initialize the engine

    let stdin = io::stdin();
    let reader = stdin.lock();

    // Loop for incoming commands
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let should_quit = uci::handle_uci_command(trimmed, &mut engine);
                if should_quit {
                    break;
                }
            }
            Err(err) => {
                eprintln!("Error reading from stdin: {}", err);
                break;
            }
        }
    }
}
