// author: Himangshu Saikia, 2018-2021 (original C++)
// Rust port: 2024
// email: himangshu.saikia.iitg@gmail.com

use chess::{Board, ChessMove, Color, MoveGen, Piece};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::book::Book;
use crate::evaluation::{eval, MATE_EVAL};

/// Maximum number of entries in the transposition table to cap memory usage.
const MAX_TT_ENTRIES: usize = 1_000_000;

/// Transposition table entry
#[derive(Clone)]
struct TTEntry {
    depth: i32,
    eval: f64,
}

/// Shared search state passed through recursion
struct SearchState {
    transposition_table: HashMap<u64, TTEntry>,
    start: Instant,
    time_limit: Duration,
    stopped: bool,
}

impl SearchState {
    fn check_time(&mut self) {
        if self.start.elapsed() > self.time_limit {
            self.stopped = true;
        }
    }
}

/// Check if a move is a capture (called BEFORE making the move)
fn is_capture(board: &Board, mv: ChessMove) -> bool {
    if board.piece_on(mv.get_dest()).is_some() {
        return true;
    }
    if let Some(ep_sq) = board.en_passant() {
        if mv.get_dest() == ep_sq {
            if let Some(piece) = board.piece_on(mv.get_source()) {
                if piece == Piece::Pawn {
                    return true;
                }
            }
        }
    }
    false
}

/// Quiescence search: only evaluate captures to avoid horizon effect
fn quiescence(board: &Board, mut alpha: f64, beta: f64, state: &mut SearchState) -> f64 {
    if state.stopped {
        return 0.0;
    }

    let stand_pat = eval(board);
    let white_to_move = board.side_to_move() == Color::White;

    if white_to_move {
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let movegen = MoveGen::new_legal(board);
        for mv in movegen {
            if !is_capture(board, mv) {
                continue;
            }
            let new_board = board.make_move_new(mv);
            let score = quiescence(&new_board, alpha, beta, state);
            if state.stopped {
                return 0.0;
            }
            if score >= beta {
                return beta;
            }
            if score > alpha {
                alpha = score;
            }
        }
        alpha
    } else {
        let mut beta = beta;
        if stand_pat <= alpha {
            return alpha;
        }
        if stand_pat < beta {
            beta = stand_pat;
        }

        let movegen = MoveGen::new_legal(board);
        for mv in movegen {
            if !is_capture(board, mv) {
                continue;
            }
            let new_board = board.make_move_new(mv);
            let score = quiescence(&new_board, alpha, beta, state);
            if state.stopped {
                return 0.0;
            }
            if score <= alpha {
                return alpha;
            }
            if score < beta {
                beta = score;
            }
        }
        beta
    }
}

/// Minimax search with alpha-beta pruning (stack-based, no tree allocation)
fn search(
    board: &Board,
    mut alpha: f64,
    mut beta: f64,
    depth: i32,
    state: &mut SearchState,
    nodes: &mut u64,
) -> f64 {
    if state.stopped {
        return 0.0;
    }

    *nodes += 1;
    // Check time every 4096 nodes
    if *nodes & 4095 == 0 {
        state.check_time();
        if state.stopped {
            return 0.0;
        }
    }

    let key = board.get_hash();

    // Check transposition table
    if let Some(entry) = state.transposition_table.get(&key) {
        if entry.depth >= depth {
            return entry.eval;
        }
    }

    // At depth 0, enter quiescence search
    if depth <= 0 {
        return quiescence(board, alpha, beta, state);
    }

    let white_to_move = board.side_to_move() == Color::White;
    let movegen = MoveGen::new_legal(board);
    let moves: Vec<ChessMove> = movegen.collect();

    // No legal moves: checkmate or stalemate
    if moves.is_empty() {
        return eval(board);
    }

    let mut best_eval = if white_to_move {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };

    for mv in &moves {
        let new_board = board.make_move_new(*mv);
        let score = search(&new_board, alpha, beta, depth - 1, state, nodes);

        if state.stopped {
            return 0.0;
        }

        if white_to_move {
            best_eval = best_eval.max(score);
            alpha = alpha.max(score);
        } else {
            best_eval = best_eval.min(score);
            beta = beta.min(score);
        }

        if beta <= alpha {
            break;
        }
    }

    // Store in transposition table
    if state.transposition_table.len() < MAX_TT_ENTRIES {
        state.transposition_table.insert(
            key,
            TTEntry {
                depth,
                eval: best_eval,
            },
        );
    }

    best_eval
}

/// Play the best move for the current position
/// Returns the best move in UCI format and the evaluation
pub fn play_move(board: &Board, book: &Book, time_to_move: f64) -> (String, f64) {
    // Try to find a random move from the book
    let pos_key = board.get_hash();

    if let Some(book_moves) = book.get(&pos_key) {
        if book_moves.len() > 1 {
            use rand::seq::SliceRandom;
            let moves: Vec<_> = book_moves.iter().collect();
            if let Some(&&chosen_move) = moves.choose(&mut rand::thread_rng()) {
                return (format!("{}", chosen_move), 0.0);
            }
        } else if let Some(&mv) = book_moves.iter().next() {
            return (format!("{}", mv), 0.0);
        }
    }

    // Generate legal moves at root
    let movegen = MoveGen::new_legal(board);
    let mut moves: Vec<(ChessMove, f64)> = movegen.map(|mv| (mv, 0.0)).collect();

    if moves.is_empty() {
        return (String::new(), 0.0);
    }

    if moves.len() == 1 {
        return (format!("{}", moves[0].0), eval(board));
    }

    // Iterative deepening
    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(time_to_move);
    let white_to_move = board.side_to_move() == Color::White;

    let mut best_move = moves[0].0;
    let mut best_eval = 0.0;
    let mut state = SearchState {
        transposition_table: HashMap::new(),
        start,
        time_limit,
        stopped: false,
    };

    for depth in 1.. {
        let mut alpha = f64::NEG_INFINITY;
        let mut beta = f64::INFINITY;
        let mut depth_best_move = moves[0].0;
        let mut depth_best_eval = if white_to_move {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut nodes: u64 = 0;

        for (mv, mv_eval) in &mut moves {
            let new_board = board.make_move_new(*mv);
            let score = search(&new_board, alpha, beta, depth - 1, &mut state, &mut nodes);

            if state.stopped {
                break;
            }

            *mv_eval = score;

            if white_to_move {
                if score > depth_best_eval {
                    depth_best_eval = score;
                    depth_best_move = *mv;
                }
                alpha = alpha.max(score);
            } else {
                if score < depth_best_eval {
                    depth_best_eval = score;
                    depth_best_move = *mv;
                }
                beta = beta.min(score);
            }
        }

        // Only update best move if this depth completed
        if !state.stopped {
            best_move = depth_best_move;
            best_eval = depth_best_eval;

            // Sort moves by eval for next iteration (best first for better pruning)
            if white_to_move {
                moves.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            } else {
                moves.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            }

            // If mate found, stop
            if best_eval.abs() == MATE_EVAL {
                break;
            }
        } else {
            break;
        }
    }

    (format!("{}", best_move), best_eval)
}

/// Set up the position from a FEN string and list of moves
pub fn set_position(fen: &str, moves: &[String]) -> Board {
    use std::str::FromStr;

    let mut board = Board::from_str(fen).unwrap_or_default();

    for move_str in moves {
        if let Ok(mv) = ChessMove::from_str(move_str) {
            if MoveGen::new_legal(&board).any(|m| m == mv) {
                board = board.make_move_new(mv);
            }
        } else if let Some(mv) = parse_uci_move(&board, move_str) {
            board = board.make_move_new(mv);
        }
    }

    board
}

/// Parse a UCI format move string (e.g., "e2e4", "e7e8q")
fn parse_uci_move(board: &Board, move_str: &str) -> Option<ChessMove> {
    use chess::{File, Rank, Square};

    if move_str.len() < 4 {
        return None;
    }

    let chars: Vec<char> = move_str.chars().collect();

    let from_file = File::from_index((chars[0] as u8 - b'a') as usize);
    let from_rank = Rank::from_index((chars[1] as u8 - b'1') as usize);
    let to_file = File::from_index((chars[2] as u8 - b'a') as usize);
    let to_rank = Rank::from_index((chars[3] as u8 - b'1') as usize);

    let from = Square::make_square(from_rank, from_file);
    let to = Square::make_square(to_rank, to_file);

    let promotion = if move_str.len() >= 5 {
        match chars[4] {
            'q' | 'Q' => Some(Piece::Queen),
            'r' | 'R' => Some(Piece::Rook),
            'b' | 'B' => Some(Piece::Bishop),
            'n' | 'N' => Some(Piece::Knight),
            _ => None,
        }
    } else {
        None
    };

    let movegen = MoveGen::new_legal(board);
    for mv in movegen {
        if mv.get_source() == from && mv.get_dest() == to {
            if let Some(promo) = promotion {
                if mv.get_promotion() == Some(promo) {
                    return Some(mv);
                }
            } else if mv.get_promotion().is_none() {
                return Some(mv);
            }
        }
    }

    let movegen = MoveGen::new_legal(board);
    let matching: Vec<_> = movegen
        .filter(|mv| mv.get_source() == from && mv.get_dest() == to)
        .collect();

    if matching.len() == 1 {
        return Some(matching[0]);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_set_position_startpos() {
        let board = set_position(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[],
        );
        assert_eq!(board, Board::default());
    }

    #[test]
    fn test_set_position_with_moves() {
        let board = set_position(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &["e2e4".to_string(), "e7e5".to_string()],
        );
        let expected =
            Board::from_str("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2")
                .unwrap();
        assert_eq!(board, expected);
    }

    #[test]
    fn test_play_move_starting() {
        let board = Board::default();
        let book = Book::new();
        let (mv, _eval) = play_move(&board, &book, 0.5);
        assert!(!mv.is_empty(), "Should find a move");
    }
}
