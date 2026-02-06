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

/// Maximum time per move in seconds
const MAX_TIME_PER_MOVE: f64 = 5.0;

fn main() {
    uci_main();
}

fn uci_main() {
    // Load the opening book
    let book = book::load_games("./engines/uci_games.txt");

    let mut board = Board::default();
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
                // Reset state - nothing special needed
                board = Board::default();
            }

            "isready" => {
                println!("readyok");
                let _ = stdout.flush();
            }

            "position" => {
                let (fen, moves) = parse_position_command(&tokens);
                board = engine::set_position(&fen, &moves);
            }

            "go" => {
                let time_to_move = parse_go_command(&tokens, &board);
                let time_to_move = time_to_move.min(MAX_TIME_PER_MOVE);

                println!("info Thinking...");
                let _ = stdout.flush();

                let (best_move, eval) = engine::play_move(&board, &book, time_to_move);
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
    let mut time_to_move = 1.0; // Default time

    // Parse time controls: go wtime X btime Y winc Z binc W
    if tokens.len() >= 9
        && tokens.get(1) == Some(&"wtime")
        && tokens.get(3) == Some(&"btime")
        && tokens.get(5) == Some(&"winc")
        && tokens.get(7) == Some(&"binc")
    {
        let wtime: i64 = tokens.get(2).and_then(|s| s.parse().ok()).unwrap_or(60000);
        let btime: i64 = tokens.get(4).and_then(|s| s.parse().ok()).unwrap_or(60000);
        let winc: i64 = tokens.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
        let binc: i64 = tokens.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);

        // Calculate time to move: (remaining_time + increment) / 60
        // This gives us roughly 1/60th of our time bank per move
        time_to_move = if board.side_to_move() == Color::White {
            (wtime + winc) as f64 / 60000.0
        } else {
            (btime + binc) as f64 / 60000.0
        };
    }

    // Also handle simpler formats
    // go movetime X (time in milliseconds)
    if let Some(idx) = tokens.iter().position(|&t| t == "movetime") {
        if let Some(time_ms) = tokens.get(idx + 1).and_then(|s| s.parse::<i64>().ok()) {
            time_to_move = time_ms as f64 / 1000.0;
        }
    }

    // go depth X (fixed depth, we'll just use a reasonable time)
    // For now, we don't implement depth-limited search differently

    time_to_move
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
        // (300000 + 3000) / 60000 = 5.05
        assert!((time - 5.05).abs() < 0.01);
    }
}
