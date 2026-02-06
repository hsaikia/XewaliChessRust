// author: Himangshu Saikia, 2018-2021 (original C++)
// Rust port: 2024
// email: himangshu.saikia.iitg@gmail.com

use chess::{Board, ChessMove, MoveGen};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Opening book: maps position hash to set of possible moves
pub type Book = HashMap<u64, HashSet<ChessMove>>;

/// Parse a UCI format move string (e.g., "e2e4", "e7e8q")
fn parse_uci_move(board: &Board, move_str: &str) -> Option<ChessMove> {
    use chess::{File as ChessFile, Piece, Rank, Square};

    let move_str = move_str.trim();
    if move_str.len() < 4 {
        return None;
    }

    let chars: Vec<char> = move_str.chars().collect();

    // Parse source square
    let from_file_idx = (chars[0] as u8).checked_sub(b'a')?;
    let from_rank_idx = (chars[1] as u8).checked_sub(b'1')?;
    if from_file_idx > 7 || from_rank_idx > 7 {
        return None;
    }

    // Parse destination square
    let to_file_idx = (chars[2] as u8).checked_sub(b'a')?;
    let to_rank_idx = (chars[3] as u8).checked_sub(b'1')?;
    if to_file_idx > 7 || to_rank_idx > 7 {
        return None;
    }

    let from_file = ChessFile::from_index(from_file_idx as usize);
    let from_rank = Rank::from_index(from_rank_idx as usize);
    let to_file = ChessFile::from_index(to_file_idx as usize);
    let to_rank = Rank::from_index(to_rank_idx as usize);

    let from = Square::make_square(from_rank, from_file);
    let to = Square::make_square(to_rank, to_file);

    // Check for promotion
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

    // Find matching legal move
    let movegen = MoveGen::new_legal(board);
    for mv in movegen {
        if mv.get_source() == from && mv.get_dest() == to {
            match (promotion, mv.get_promotion()) {
                (Some(p), Some(mp)) if p == mp => return Some(mv),
                (None, None) => return Some(mv),
                (None, Some(_)) => {
                    // Move string didn't specify promotion but move has one
                    // This can happen if the move string is incomplete
                    // Accept it if it's the only matching move
                }
                _ => continue,
            }
        }
    }

    // Fallback: find any move matching from/to
    let mut movegen = MoveGen::new_legal(board);
    movegen.find(|&mv| mv.get_source() == from && mv.get_dest() == to)
}

/// Load opening book from a UCI games file
/// Each line in the file should be a sequence of UCI moves (e.g., "e2e4 e7e5 g1f3 ...")
pub fn load_games(game_file: &str) -> Book {
    let mut book = Book::new();

    let file = match File::open(game_file) {
        Ok(f) => f,
        Err(_) => {
            // Book file not found, return empty book
            return book;
        }
    };

    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let mut board = Board::default();

        for move_str in line.split_whitespace() {
            if let Some(mv) = parse_uci_move(&board, move_str) {
                let key = board.get_hash();
                book.entry(key).or_default().insert(mv);
                board = board.make_move_new(mv);
            } else {
                // Invalid move, skip rest of line
                break;
            }
        }
    }

    book
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parse_uci_move() {
        let board = Board::default();
        let mv = parse_uci_move(&board, "e2e4");
        assert!(mv.is_some());
        let mv = mv.unwrap();
        assert_eq!(format!("{}", mv), "e2e4");
    }

    #[test]
    fn test_parse_uci_promotion() {
        // Position where a pawn can promote
        let board = Board::from_str("8/P7/8/8/8/8/8/4K2k w - - 0 1").unwrap();
        let mv = parse_uci_move(&board, "a7a8q");
        assert!(mv.is_some());
    }

    #[test]
    fn test_empty_book() {
        let book = load_games("nonexistent_file.txt");
        assert!(book.is_empty());
    }
}
