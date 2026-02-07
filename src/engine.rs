// author: Himangshu Saikia, 2018-2021 (original C++)
// Rust port: 2024
// email: himangshu.saikia.iitg@gmail.com

use chess::{Board, ChessMove, Color, MoveGen, Piece, EMPTY};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::book::Book;
use crate::evaluation::{eval, MATE_EVAL};

/// Maximum number of entries in the transposition table to cap memory usage.
const MAX_TT_ENTRIES: usize = 1_000_000;

/// Maximum depth for quiescence search to prevent infinite capture chains.
const MAX_QUIESCENCE_DEPTH: i32 = 8;

/// Null-move pruning reduction
const NULL_MOVE_R: i32 = 2;

/// Transposition table bound type
#[derive(Clone, Copy, PartialEq)]
enum TTFlag {
    Exact,
    LowerBound,
    UpperBound,
}

/// Transposition table entry
#[derive(Clone)]
struct TTEntry {
    depth: i32,
    eval: f64,
    flag: TTFlag,
    best_move: Option<ChessMove>,
}

/// Shared search state passed through recursion
struct SearchState {
    transposition_table: HashMap<u64, TTEntry>,
    position_history: Vec<u64>,
    start: Instant,
    time_limit: Duration,
    nodes: u64,
    stopped: bool,
}

impl SearchState {
    fn check_time(&mut self) {
        self.nodes += 1;
        if self.nodes & 4095 == 0 && self.start.elapsed() > self.time_limit {
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
fn quiescence(
    board: &Board,
    mut alpha: f64,
    beta: f64,
    qs_depth: i32,
    state: &mut SearchState,
) -> f64 {
    if state.stopped {
        return 0.0;
    }
    state.check_time();
    if state.stopped {
        return 0.0;
    }

    let stand_pat = eval(board);

    if qs_depth >= MAX_QUIESCENCE_DEPTH {
        return stand_pat;
    }

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
            let score = quiescence(&new_board, alpha, beta, qs_depth + 1, state);
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
            let score = quiescence(&new_board, alpha, beta, qs_depth + 1, state);
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

/// Get the material value of a piece for move ordering
fn piece_order_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => 100,
        Piece::Knight => 320,
        Piece::Bishop => 330,
        Piece::Rook => 500,
        Piece::Queen => 900,
        Piece::King => 20000,
    }
}

/// Score a move for ordering. Higher scores are searched first.
fn score_move(board: &Board, mv: ChessMove, tt_move: Option<ChessMove>) -> i32 {
    // TT best move gets highest priority
    if tt_move == Some(mv) {
        return 100_000;
    }

    let mut score = 0;

    // Promotions
    if let Some(promo) = mv.get_promotion() {
        score += 9000 + piece_order_value(promo);
    }

    // Captures scored by MVV-LVA
    if let Some(victim) = board.piece_on(mv.get_dest()) {
        let attacker = board.piece_on(mv.get_source()).unwrap_or(Piece::Pawn);
        score += piece_order_value(victim) * 10 - piece_order_value(attacker);
    } else if let Some(ep_sq) = board.en_passant() {
        if mv.get_dest() == ep_sq {
            if let Some(piece) = board.piece_on(mv.get_source()) {
                if piece == Piece::Pawn {
                    score += 100 * 10 - 100; // pawn captures pawn
                }
            }
        }
    }

    score
}

/// Check if a side has non-pawn material (used for null-move pruning safety)
fn has_non_pawn_material(board: &Board, color: Color) -> bool {
    let our_pieces = *board.color_combined(color);
    let knights = *board.pieces(Piece::Knight) & our_pieces;
    let bishops = *board.pieces(Piece::Bishop) & our_pieces;
    let rooks = *board.pieces(Piece::Rook) & our_pieces;
    let queens = *board.pieces(Piece::Queen) & our_pieces;
    (knights | bishops | rooks | queens) != EMPTY
}

/// Negamax search with alpha-beta pruning, null-move pruning, and LMR
fn search(
    board: &Board,
    mut alpha: f64,
    mut beta: f64,
    depth: i32,
    allow_null: bool,
    state: &mut SearchState,
) -> f64 {
    if state.stopped {
        return 0.0;
    }
    state.check_time();
    if state.stopped {
        return 0.0;
    }

    let key = board.get_hash();

    // Repetition detection: need position to appear 2+ times in history for 3-fold
    if state.position_history.iter().filter(|&&h| h == key).count() >= 2 {
        return 0.0;
    }

    // Probe transposition table
    let mut tt_move: Option<ChessMove> = None;
    if let Some(entry) = state.transposition_table.get(&key) {
        tt_move = entry.best_move;
        if entry.depth >= depth {
            match entry.flag {
                TTFlag::Exact => return entry.eval,
                TTFlag::LowerBound => {
                    if entry.eval >= beta {
                        return entry.eval;
                    }
                }
                TTFlag::UpperBound => {
                    if entry.eval <= alpha {
                        return entry.eval;
                    }
                }
            }
        }
    }

    // At depth 0, enter quiescence search
    if depth <= 0 {
        return quiescence(board, alpha, beta, 0, state);
    }

    let white_to_move = board.side_to_move() == Color::White;
    let in_check = *board.checkers() != EMPTY;

    // Null-move pruning
    if allow_null && !in_check && depth >= 3 && has_non_pawn_material(board, board.side_to_move()) {
        if let Some(null_board) = board.null_move() {
            let null_score = search(
                &null_board,
                alpha,
                beta,
                depth - 1 - NULL_MOVE_R,
                false,
                state,
            );
            if state.stopped {
                return 0.0;
            }
            // Beta cutoff: if even passing gives a score >= beta, this position is too good
            if white_to_move && null_score >= beta {
                return beta;
            }
            if !white_to_move && null_score <= alpha {
                return alpha;
            }
        }
    }

    let movegen = MoveGen::new_legal(board);
    let mut moves: Vec<ChessMove> = movegen.collect();

    // No legal moves: checkmate or stalemate
    if moves.is_empty() {
        return eval(board);
    }

    // Move ordering: score and sort moves
    let mut scored_moves: Vec<(ChessMove, i32)> = moves
        .iter()
        .map(|&mv| (mv, score_move(board, mv, tt_move)))
        .collect();
    scored_moves.sort_by(|a, b| b.1.cmp(&a.1));
    moves = scored_moves.into_iter().map(|(mv, _)| mv).collect();

    let original_alpha = alpha;
    let original_beta = beta;
    let mut best_eval = if white_to_move {
        f64::NEG_INFINITY
    } else {
        f64::INFINITY
    };
    let mut best_move = moves[0];

    for (i, mv) in moves.iter().enumerate() {
        let capture = is_capture(board, *mv);
        let is_promotion = mv.get_promotion().is_some();
        let new_board = board.make_move_new(*mv);
        state.position_history.push(key);

        // Late Move Reductions
        let mut score;
        let gives_check = *new_board.checkers() != EMPTY;
        let do_lmr = i >= 4 && depth >= 3 && !capture && !in_check && !is_promotion && !gives_check;

        if do_lmr {
            // Reduced depth search
            score = search(&new_board, alpha, beta, depth - 2, true, state);
            if state.stopped {
                state.position_history.pop();
                return 0.0;
            }
            // Re-search at full depth if reduced search improves alpha
            let needs_research = if white_to_move {
                score > alpha
            } else {
                score < beta
            };
            if needs_research {
                score = search(&new_board, alpha, beta, depth - 1, true, state);
            }
        } else {
            score = search(&new_board, alpha, beta, depth - 1, true, state);
        }

        state.position_history.pop();

        if state.stopped {
            return 0.0;
        }

        if white_to_move {
            if score > best_eval {
                best_eval = score;
                best_move = *mv;
            }
            alpha = alpha.max(score);
        } else {
            if score < best_eval {
                best_eval = score;
                best_move = *mv;
            }
            beta = beta.min(score);
        }

        if beta <= alpha {
            break;
        }
    }

    // Determine TT flag based on relationship to original alpha/beta window
    let tt_flag = if white_to_move {
        if best_eval <= original_alpha {
            TTFlag::UpperBound
        } else if best_eval >= original_beta {
            TTFlag::LowerBound
        } else {
            TTFlag::Exact
        }
    } else if best_eval >= original_beta {
        TTFlag::UpperBound
    } else if best_eval <= original_alpha {
        TTFlag::LowerBound
    } else {
        TTFlag::Exact
    };

    // Store in transposition table
    if state.transposition_table.len() < MAX_TT_ENTRIES {
        state.transposition_table.insert(
            key,
            TTEntry {
                depth,
                eval: best_eval,
                flag: tt_flag,
                best_move: Some(best_move),
            },
        );
    }

    best_eval
}

/// Play the best move for the current position
/// Returns the best move in UCI format and the evaluation
pub fn play_move(board: &Board, book: &Book, time_to_move: f64, history: &[u64]) -> (String, f64) {
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
        position_history: history.to_vec(),
        start,
        time_limit,
        nodes: 0,
        stopped: false,
    };

    for depth in 1.. {
        let mut depth_best_move = moves[0].0;
        let mut depth_best_eval = if white_to_move {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };

        for (mv, mv_eval) in &mut moves {
            let new_board = board.make_move_new(*mv);
            let score = search(
                &new_board,
                f64::NEG_INFINITY,
                f64::INFINITY,
                depth - 1,
                true,
                &mut state,
            );

            if state.stopped {
                break;
            }

            *mv_eval = score;

            if white_to_move {
                if score > depth_best_eval {
                    depth_best_eval = score;
                    depth_best_move = *mv;
                }
            } else if score < depth_best_eval {
                depth_best_eval = score;
                depth_best_move = *mv;
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
/// Returns the board and a history of position hashes (for repetition detection)
pub fn set_position(fen: &str, moves: &[String]) -> (Board, Vec<u64>) {
    use std::str::FromStr;

    let mut board = Board::from_str(fen).unwrap_or_default();
    let mut history = vec![board.get_hash()];

    for move_str in moves {
        if let Ok(mv) = ChessMove::from_str(move_str) {
            if MoveGen::new_legal(&board).any(|m| m == mv) {
                board = board.make_move_new(mv);
                history.push(board.get_hash());
            }
        } else if let Some(mv) = parse_uci_move(&board, move_str) {
            board = board.make_move_new(mv);
            history.push(board.get_hash());
        }
    }

    (board, history)
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
        let (board, history) = set_position(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &[],
        );
        assert_eq!(board, Board::default());
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_set_position_with_moves() {
        let (board, history) = set_position(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &["e2e4".to_string(), "e7e5".to_string()],
        );
        let expected =
            Board::from_str("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq e6 0 2")
                .unwrap();
        assert_eq!(board, expected);
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_play_move_starting() {
        let board = Board::default();
        let book = Book::new();
        let history = vec![board.get_hash()];
        let (mv, _eval) = play_move(&board, &book, 0.5, &history);
        assert!(!mv.is_empty(), "Should find a move");
    }
}
