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

/// A node in the search tree
struct MoveNode {
    mv: Option<ChessMove>,
    eval: f64,
    children: Vec<MoveNode>,
}

impl MoveNode {
    fn new(mv: Option<ChessMove>, white_to_move: bool) -> Self {
        MoveNode {
            mv,
            eval: if white_to_move {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            children: Vec::new(),
        }
    }

    fn new_root() -> Self {
        MoveNode {
            mv: None,
            eval: 0.0,
            children: Vec::new(),
        }
    }
}

/// Check if a move is a capture (called BEFORE making the move)
fn is_capture(board: &Board, mv: ChessMove) -> bool {
    // Check if destination has a piece
    if board.piece_on(mv.get_dest()).is_some() {
        return true;
    }
    // Check for en passant
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

/// Populate the next moves for a node
/// board_after_move: the board state AFTER node's move has been made
/// only_captures: if true, only add captures (for quiescence search)
/// last_capture_dest: if in quiescence mode, only recaptures to this square
fn populate_next_moves(
    node: &mut MoveNode,
    board_after_move: &Board,
    only_captures: bool,
    last_capture_dest: Option<chess::Square>,
) {
    let white_to_move = board_after_move.side_to_move() == Color::White;
    let movegen = MoveGen::new_legal(board_after_move);

    for mv in movegen {
        if only_captures {
            // Must be a capture
            if !is_capture(board_after_move, mv) {
                continue;
            }

            // If there was a previous capture, only allow recaptures to that square
            if let Some(last_sq) = last_capture_dest {
                if mv.get_dest() != last_sq {
                    continue;
                }
            }
        }

        // Check if this move is already in children
        let already_exists = node.children.iter().any(|child| child.mv == Some(mv));
        if already_exists {
            continue;
        }

        node.children.push(MoveNode::new(Some(mv), white_to_move));
    }
}

/// Order moves by their evaluation (best first)
fn order_moves(node: &mut MoveNode, white_to_move: bool) {
    if white_to_move {
        node.children.sort_by(|a, b| {
            b.eval
                .partial_cmp(&a.eval)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        node.children.sort_by(|a, b| {
            a.eval
                .partial_cmp(&b.eval)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

/// Minimax with alpha-beta pruning
/// board_before_move: the board state BEFORE this node's move
fn minimax(
    node: &mut MoveNode,
    board_before_move: &Board,
    alpha: f64,
    beta: f64,
    depth: i32,
    transposition_table: &mut HashMap<u64, TTEntry>,
    transpositions: &mut i32,
) {
    // First, make the move (if any) to get the board state after this node's move
    let board_after_move = if let Some(mv) = node.mv {
        board_before_move.make_move_new(mv)
    } else {
        *board_before_move
    };

    // Determine if we're in quiescence search
    let only_captures = depth == 0;

    // For quiescence, track the destination of the last capture
    let last_capture_dest = if only_captures {
        if let Some(mv) = node.mv {
            if is_capture(board_before_move, mv) {
                Some(mv.get_dest())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Populate children based on the board AFTER this node's move
    populate_next_moves(node, &board_after_move, only_captures, last_capture_dest);

    // Get the zobrist hash key for the position after the move
    let key = board_after_move.get_hash();

    // Check transposition table
    if let Some(entry) = transposition_table.get(&key) {
        if entry.depth >= depth {
            node.eval = entry.eval;
            *transpositions += 1;
            return;
        }
    }

    // Terminal position reached, call static evaluation function
    if node.children.is_empty() {
        node.eval = eval(&board_after_move);
        if transposition_table.len() < MAX_TT_ENTRIES {
            transposition_table.insert(
                key,
                TTEntry {
                    depth,
                    eval: node.eval,
                },
            );
        }
    } else {
        // Recurse
        let white_to_move = board_after_move.side_to_move() == Color::White;
        node.eval = if white_to_move {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };

        let mut current_alpha = alpha;
        let mut current_beta = beta;

        for child in &mut node.children {
            // Pass board_after_move as the "before" board for the child
            minimax(
                child,
                &board_after_move,
                current_alpha,
                current_beta,
                (depth - 1).max(0),
                transposition_table,
                transpositions,
            );

            let child_eval = child.eval;

            if white_to_move {
                node.eval = node.eval.max(child_eval);
                current_alpha = current_alpha.max(child_eval);
            } else {
                node.eval = node.eval.min(child_eval);
                current_beta = current_beta.min(child_eval);
            }

            // Alpha-beta cutoff
            if current_beta < current_alpha {
                break;
            }
        }

        // Store in transposition table
        if transposition_table.len() < MAX_TT_ENTRIES {
            transposition_table.insert(
                key,
                TTEntry {
                    depth,
                    eval: node.eval,
                },
            );
        }

        // Order child moves for next iteration
        order_moves(node, white_to_move);
    }
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

    // Iterative deepening
    let start = Instant::now();
    let time_limit = Duration::from_secs_f64(time_to_move);

    let mut root = MoveNode::new_root();
    let mut best_move = String::new();
    let mut best_eval = 0.0;

    for depth in 1.. {
        let mut transposition_table = HashMap::new();
        let mut transpositions = 0;

        minimax(
            &mut root,
            board,
            f64::NEG_INFINITY,
            f64::INFINITY,
            depth,
            &mut transposition_table,
            &mut transpositions,
        );

        if !root.children.is_empty() {
            best_move = format!("{}", root.children[0].mv.unwrap());
            best_eval = root.children[0].eval;

            // If mate found, no need to search deeper
            if best_eval.abs() == MATE_EVAL {
                break;
            }

            // If only one move, no point searching deeper
            if root.children.len() == 1 {
                break;
            }
        }

        // Check time
        if start.elapsed() > time_limit {
            break;
        }
    }

    (best_move, best_eval)
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
