// author: Himangshu Saikia, 2018-2021 (original C++)
// Rust port: 2024
// email: himangshu.saikia.iitg@gmail.com

mod book;
mod engine;
mod evaluation;

use chess::{Board, Color};
use std::io::{self, BufRead, Write};

/// The starting position FEN
const START_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn main() {
    uci_main();
}

fn uci_main() {
    // Load the opening book
    let book = book::load_games("./book/uci_games.txt");

    let mut board = Board::default();
    let mut position_history: Vec<u64> = vec![board.get_hash()];
    let mut current_evaluation = 0.0;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let tokens: Vec<&str> = line.split_whitespace().collect();

        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "uci" => {
                println!("id name Xewali 1.0");
                println!("id author Himangshu Saikia");
                println!("uciok");
                let _ = stdout.flush();
            }

            "ucinewgame" => {
                board = Board::default();
                position_history = vec![board.get_hash()];
            }

            "isready" => {
                println!("readyok");
                let _ = stdout.flush();
            }

            "position" => {
                let (fen, moves) = parse_position_command(&tokens);
                let result = engine::set_position(&fen, &moves);
                board = result.0;
                position_history = result.1;
            }

            "go" => {
                let time_to_move = parse_go_command(&tokens, &board);

                println!("info Thinking...");
                let _ = stdout.flush();

                let (best_move, eval) =
                    engine::play_move(&board, &book, time_to_move, &position_history);
                current_evaluation = eval;

                println!("bestmove {}", best_move);
                let _ = stdout.flush();
            }

            "quit" => {
                break;
            }

            "eval" => {
                // Custom command to show current evaluation
                println!("{}", current_evaluation);
                let _ = stdout.flush();
            }

            "d" | "display" => {
                // Debug: display the current board
                println!("{}", board);
                let _ = stdout.flush();
            }

            _ => {
                // Unknown command, ignore
            }
        }
    }
}

/// Parse the "position" command and return (fen, moves)
fn parse_position_command(tokens: &[&str]) -> (String, Vec<String>) {
    if tokens.len() < 2 {
        return (START_POSITION.to_string(), vec![]);
    }

    let mut fen = String::new();
    let mut moves = Vec::new();
    let mut reading_fen = true;

    if tokens[1] == "startpos" {
        fen = START_POSITION.to_string();
        reading_fen = false;
    } else if tokens[1] == "fen" {
        // FEN will be constructed from subsequent tokens
    }

    let start_idx = if tokens[1] == "startpos" || tokens[1] == "fen" {
        2
    } else {
        1
    };

    for token in tokens.iter().skip(start_idx) {
        if *token == "moves" {
            reading_fen = false;
            continue;
        }

        if reading_fen {
            if !fen.is_empty() {
                fen.push(' ');
            }
            fen.push_str(token);
        } else {
            moves.push(token.to_string());
        }
    }

    // If no FEN was provided (shouldn't happen), use start position
    if fen.is_empty() {
        fen = START_POSITION.to_string();
    }

    (fen, moves)
}

/// Parse the "go" command and return the time to move in seconds
fn parse_go_command(tokens: &[&str], board: &Board) -> f64 {
    // Helper to find a value after a named token
    let find_value = |name: &str| -> Option<i64> {
        tokens
            .iter()
            .position(|&t| t == name)
            .and_then(|i| tokens.get(i + 1))
            .and_then(|s| s.parse().ok())
    };

    // go movetime X (time in milliseconds) — takes priority
    if let Some(time_ms) = find_value("movetime") {
        return time_ms as f64 / 1000.0;
    }

    // Parse time controls: go wtime X btime Y [winc Z] [binc W]
    let (remaining, inc) = if board.side_to_move() == Color::White {
        (find_value("wtime"), find_value("winc"))
    } else {
        (find_value("btime"), find_value("binc"))
    };

    if let Some(remaining_ms) = remaining {
        let inc_ms = inc.unwrap_or(0);
        // Allocate roughly 1/30th of remaining time + increment
        return (remaining_ms as f64 / 30000.0) + (inc_ms as f64 / 1000.0);
    }

    // Default fallback
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_position_startpos() {
        let tokens = vec!["position", "startpos"];
        let (fen, moves) = parse_position_command(&tokens);
        assert_eq!(fen, START_POSITION);
        assert!(moves.is_empty());
    }

    #[test]
    fn test_parse_position_startpos_with_moves() {
        let tokens = vec!["position", "startpos", "moves", "e2e4", "e7e5"];
        let (fen, moves) = parse_position_command(&tokens);
        assert_eq!(fen, START_POSITION);
        assert_eq!(moves, vec!["e2e4", "e7e5"]);
    }

    #[test]
    fn test_parse_position_fen() {
        let tokens = vec![
            "position",
            "fen",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR",
            "b",
            "KQkq",
            "-",
            "0",
            "1",
        ];
        let (fen, moves) = parse_position_command(&tokens);
        assert_eq!(
            fen,
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"
        );
        assert!(moves.is_empty());
    }

    #[test]
    fn test_parse_go_command() {
        let board = Board::default();
        let tokens = vec![
            "go", "wtime", "300000", "btime", "300000", "winc", "3000", "binc", "3000",
        ];
        let time = parse_go_command(&tokens, &board);
        // 300000 / 30000 + 3000 / 1000 = 10 + 3 = 13
        assert!((time - 13.0).abs() < 0.01);
    }
}
