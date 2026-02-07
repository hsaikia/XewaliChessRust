// author: Himangshu Saikia, 2018-2021 (original C++)
// Rust port: 2024
// email: himangshu.saikia.iitg@gmail.com

use chess::{BitBoard, Board, BoardStatus, Color, File, Piece, Rank, Square, EMPTY};

/// Mate evaluation score
pub const MATE_EVAL: f64 = 1e6;

/// Piece values
pub const KING_VAL: i32 = 20000;
pub const QUEEN_VAL: i32 = 900;
pub const ROOK_VAL: i32 = 500;
pub const BISHOP_VAL: i32 = 330;
pub const KNIGHT_VAL: i32 = 320;
pub const PAWN_VAL: i32 = 100;

/// Material threshold for endgame detection
const ENDGAME_THRESHOLD: i32 = 2000;

// Piece-square tables (from White's perspective at the bottom, index 0 = A1)
// The chess crate uses A1=0, H1=7, A8=56, H8=63

/// White Pawn table (A1=0 ... H8=63)
#[rustfmt::skip]
const WHITE_PAWN_TABLE: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10,-20,-20, 10, 10,  5,
    5, -5,-10,  0,  0,-10, -5,  5,
    0,  0,  0, 20, 20,  0,  0,  0,
    5,  5, 10, 25, 25, 10,  5,  5,
   10, 10, 20, 30, 30, 20, 10, 10,
   50, 50, 50, 50, 50, 50, 50, 50,
    0,  0,  0,  0,  0,  0,  0,  0,
];

/// Black Pawn table
#[rustfmt::skip]
const BLACK_PAWN_TABLE: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
   50, 50, 50, 50, 50, 50, 50, 50,
   10, 10, 20, 30, 30, 20, 10, 10,
    5,  5, 10, 25, 25, 10,  5,  5,
    0,  0,  0, 20, 20,  0,  0,  0,
    5, -5,-10,  0,  0,-10, -5,  5,
    5, 10, 10,-20,-20, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
];

/// White Knight table
#[rustfmt::skip]
const WHITE_KNIGHT_TABLE: [i32; 64] = [
   -50,-40,-30,-30,-30,-30,-40,-50,
   -40,-20,  0,  5,  5,  0,-20,-40,
   -30,  5, 10, 15, 15, 10,  5,-30,
   -30,  0, 15, 20, 20, 15,  0,-30,
   -30,  5, 15, 20, 20, 15,  5,-30,
   -30,  0, 10, 15, 15, 10,  0,-30,
   -40,-20,  0,  0,  0,  0,-20,-40,
   -50,-40,-30,-30,-30,-30,-40,-50,
];

/// Black Knight table
#[rustfmt::skip]
const BLACK_KNIGHT_TABLE: [i32; 64] = [
   -50,-40,-30,-30,-30,-30,-40,-50,
   -40,-20,  0,  0,  0,  0,-20,-40,
   -30,  0, 10, 15, 15, 10,  0,-30,
   -30,  5, 15, 20, 20, 15,  5,-30,
   -30,  0, 15, 20, 20, 15,  0,-30,
   -30,  0, 10, 15, 15, 10,  0,-30,
   -40,-20,  0,  5,  5,  0,-20,-40,
   -50,-40,-30,-30,-30,-30,-40,-50,
];

/// White Bishop table
#[rustfmt::skip]
const WHITE_BISHOP_TABLE: [i32; 64] = [
   -20,-10,-10,-10,-10,-10,-10,-20,
   -10,  5,  0,  0,  0,  0,  5,-10,
   -10, 10, 10, 10, 10, 10, 10,-10,
   -10,  0, 10, 10, 10, 10,  0,-10,
   -10,  5,  5, 10, 10,  5,  5,-10,
   -10,  0,  5, 10, 10,  5,  0,-10,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -20,-10,-10,-10,-10,-10,-10,-20,
];

/// Black Bishop table
#[rustfmt::skip]
const BLACK_BISHOP_TABLE: [i32; 64] = [
   -20,-10,-10,-10,-10,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5, 10, 10,  5,  0,-10,
   -10,  5,  5, 10, 10,  5,  5,-10,
   -10,  0, 10, 10, 10, 10,  0,-10,
   -10, 10, 10, 10, 10, 10, 10,-10,
   -10,  5,  0,  0,  0,  0,  5,-10,
   -20,-10,-10,-10,-10,-10,-10,-20,
];

/// White Rook table
#[rustfmt::skip]
const WHITE_ROOK_TABLE: [i32; 64] = [
    0,  0,  0,  5,  5,  0,  0,  0,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    5, 10, 10, 10, 10, 10, 10,  5,
    0,  0,  0,  0,  0,  0,  0,  0,
];

/// Black Rook table
#[rustfmt::skip]
const BLACK_ROOK_TABLE: [i32; 64] = [
    0,  0,  0,  0,  0,  0,  0,  0,
    5, 10, 10, 10, 10, 10, 10,  5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
   -5,  0,  0,  0,  0,  0,  0, -5,
    0,  0,  0,  5,  5,  0,  0,  0,
];

/// White Queen table
#[rustfmt::skip]
const WHITE_QUEEN_TABLE: [i32; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -10,  5,  5,  5,  5,  5,  0,-10,
     0,  0,  5,  5,  5,  5,  0, -5,
    -5,  0,  5,  5,  5,  5,  0, -5,
   -10,  0,  5,  5,  5,  5,  0,-10,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20,
];

/// Black Queen table
#[rustfmt::skip]
const BLACK_QUEEN_TABLE: [i32; 64] = [
   -20,-10,-10, -5, -5,-10,-10,-20,
   -10,  0,  0,  0,  0,  0,  0,-10,
   -10,  0,  5,  5,  5,  5,  0,-10,
    -5,  0,  5,  5,  5,  5,  0, -5,
     0,  0,  5,  5,  5,  5,  0, -5,
   -10,  5,  5,  5,  5,  5,  0,-10,
   -10,  0,  5,  0,  0,  0,  0,-10,
   -20,-10,-10, -5, -5,-10,-10,-20,
];

/// White King Middlegame table
#[rustfmt::skip]
const WHITE_KING_MG_TABLE: [i32; 64] = [
    20, 30, 10,  0,  0, 10, 30, 20,
    20, 20,  0,  0,  0,  0, 20, 20,
   -10,-20,-20,-20,-20,-20,-20,-10,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
];

/// Black King Middlegame table
#[rustfmt::skip]
const BLACK_KING_MG_TABLE: [i32; 64] = [
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -30,-40,-40,-50,-50,-40,-40,-30,
   -20,-30,-30,-40,-40,-30,-30,-20,
   -10,-20,-20,-20,-20,-20,-20,-10,
    20, 20,  0,  0,  0,  0, 20, 20,
    20, 30, 10,  0,  0, 10, 30, 20,
];

/// White King Endgame table
#[rustfmt::skip]
const WHITE_KING_EG_TABLE: [i32; 64] = [
   -50,-30,-30,-30,-30,-30,-30,-50,
   -30,-30,  0,  0,  0,  0,-30,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-20,-10,  0,  0,-10,-20,-30,
   -50,-40,-30,-20,-20,-30,-40,-50,
];

/// Black King Endgame table
#[rustfmt::skip]
const BLACK_KING_EG_TABLE: [i32; 64] = [
   -50,-40,-30,-20,-20,-30,-40,-50,
   -30,-20,-10,  0,  0,-10,-20,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 30, 40, 40, 30,-10,-30,
   -30,-10, 20, 30, 30, 20,-10,-30,
   -30,-30,  0,  0,  0,  0,-30,-30,
   -50,-30,-30,-30,-30,-30,-30,-50,
];

/// Get piece-square table value for a piece at a square
fn piece_square_value(piece: Piece, color: Color, square: Square, is_endgame: bool) -> i32 {
    let sq_idx = square.to_index();

    match (piece, color) {
        (Piece::Pawn, Color::White) => WHITE_PAWN_TABLE[sq_idx],
        (Piece::Pawn, Color::Black) => BLACK_PAWN_TABLE[sq_idx],
        (Piece::Knight, Color::White) => WHITE_KNIGHT_TABLE[sq_idx],
        (Piece::Knight, Color::Black) => BLACK_KNIGHT_TABLE[sq_idx],
        (Piece::Bishop, Color::White) => WHITE_BISHOP_TABLE[sq_idx],
        (Piece::Bishop, Color::Black) => BLACK_BISHOP_TABLE[sq_idx],
        (Piece::Rook, Color::White) => WHITE_ROOK_TABLE[sq_idx],
        (Piece::Rook, Color::Black) => BLACK_ROOK_TABLE[sq_idx],
        (Piece::Queen, Color::White) => WHITE_QUEEN_TABLE[sq_idx],
        (Piece::Queen, Color::Black) => BLACK_QUEEN_TABLE[sq_idx],
        (Piece::King, Color::White) => {
            if is_endgame {
                WHITE_KING_EG_TABLE[sq_idx]
            } else {
                WHITE_KING_MG_TABLE[sq_idx]
            }
        }
        (Piece::King, Color::Black) => {
            if is_endgame {
                BLACK_KING_EG_TABLE[sq_idx]
            } else {
                BLACK_KING_MG_TABLE[sq_idx]
            }
        }
    }
}

/// Get the base material value for a piece type
fn piece_value(piece: Piece) -> i32 {
    match piece {
        Piece::Pawn => PAWN_VAL,
        Piece::Knight => KNIGHT_VAL,
        Piece::Bishop => BISHOP_VAL,
        Piece::Rook => ROOK_VAL,
        Piece::Queen => QUEEN_VAL,
        Piece::King => KING_VAL,
    }
}

/// Count bits in a bitboard (mobility)
fn count_bits(bb: BitBoard) -> i32 {
    bb.popcnt() as i32
}

/// Game result type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GameResult {
    Ongoing,
    WhiteWins,
    BlackWins,
    Draw,
}

/// Check if the game has ended and return the result
pub fn has_game_ended(board: &Board) -> GameResult {
    match board.status() {
        BoardStatus::Checkmate => {
            // The side to move is in checkmate, so they lost
            if board.side_to_move() == Color::White {
                GameResult::BlackWins
            } else {
                GameResult::WhiteWins
            }
        }
        BoardStatus::Stalemate => GameResult::Draw,
        BoardStatus::Ongoing => {
            // Check for insufficient material or other draw conditions
            // The chess crate handles 50-move rule and threefold repetition
            // via the Game struct, but for position-only we check material
            if is_insufficient_material(board) {
                GameResult::Draw
            } else {
                GameResult::Ongoing
            }
        }
    }
}

/// Check for insufficient material to mate
fn is_insufficient_material(board: &Board) -> bool {
    let all_pieces = *board.combined();
    let piece_count = all_pieces.popcnt();

    // King vs King
    if piece_count == 2 {
        return true;
    }

    // King + minor piece vs King
    if piece_count == 3 {
        let knights = *board.pieces(Piece::Knight);
        let bishops = *board.pieces(Piece::Bishop);
        if knights.popcnt() == 1 || bishops.popcnt() == 1 {
            return true;
        }
    }

    false
}

/// Calculate material for one side (without piece-square tables)
fn calculate_material(board: &Board, color: Color) -> i32 {
    let mut material = 0;

    for piece in [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
    ] {
        let piece_bb = *board.pieces(piece) & *board.color_combined(color);
        material += piece_bb.popcnt() as i32 * piece_value(piece);
    }

    material
}

/// Evaluate the position
/// Returns positive values for White advantage, negative for Black advantage
pub fn eval(board: &Board) -> f64 {
    // Check for game end
    match has_game_ended(board) {
        GameResult::WhiteWins => return MATE_EVAL,
        GameResult::BlackWins => return -MATE_EVAL,
        GameResult::Draw => return 0.0,
        GameResult::Ongoing => {}
    }

    let mut white_material: i32 = 0;
    let mut black_material: i32 = 0;

    // Calculate raw material (without king) for endgame detection
    let white_raw_material = calculate_material(board, Color::White);
    let black_raw_material = calculate_material(board, Color::Black);
    let is_endgame =
        white_raw_material < ENDGAME_THRESHOLD && black_raw_material < ENDGAME_THRESHOLD;

    // Calculate material with piece-square tables
    for color in [Color::White, Color::Black] {
        for piece in [
            Piece::Pawn,
            Piece::Knight,
            Piece::Bishop,
            Piece::Rook,
            Piece::Queen,
            Piece::King,
        ] {
            let piece_bb = *board.pieces(piece) & *board.color_combined(color);

            for sq in piece_bb {
                let base_value = if piece == Piece::King {
                    0
                } else {
                    piece_value(piece)
                };
                let psq_value = piece_square_value(piece, color, sq, is_endgame);

                if color == Color::White {
                    white_material += base_value + psq_value;
                } else {
                    black_material += base_value + psq_value;
                }
            }
        }
    }

    // Calculate mobility (influence)
    let white_influence = calculate_mobility(board, Color::White);
    let black_influence = calculate_mobility(board, Color::Black);

    // Avoid division by zero
    let influence_ratio = if black_influence > 0 {
        white_influence as f64 / black_influence as f64
    } else if white_influence > 0 {
        10.0 // White has all the influence
    } else {
        1.0 // No influence from either side
    };

    // King safety (skipped in endgame)
    let king_safety_score = if !is_endgame {
        king_safety(board, Color::White, is_endgame) - king_safety(board, Color::Black, is_endgame)
    } else {
        0
    };

    // Final evaluation: material difference + mobility bonus + king safety
    (white_material - black_material + king_safety_score) as f64 + 10.0 * influence_ratio.ln()
}

/// Build a bitboard mask for all squares on a given file.
fn file_mask(file: File) -> BitBoard {
    let mut bb = EMPTY;
    for rank_idx in 0..8 {
        bb |= BitBoard::set(Rank::from_index(rank_idx), file);
    }
    bb
}

/// Evaluate king safety for one side. Returns a score in centipawns (positive = safer).
/// In the endgame this returns 0, since king centralization matters more than shelter.
///
/// Components:
///   - Pawn shield: bonus for friendly pawns on the 2nd/3rd rank near the king
///   - Open files: penalty for missing pawns on files near the king
///   - Enemy attacks: penalty for enemy pieces attacking squares around the king
fn king_safety(board: &Board, color: Color, is_endgame: bool) -> i32 {
    if is_endgame {
        return 0;
    }

    let king_sq = board.king_square(color);
    let king_file = king_sq.get_file().to_index() as i32;
    let enemy = if color == Color::White {
        Color::Black
    } else {
        Color::White
    };

    let our_pawns = *board.pieces(Piece::Pawn) & *board.color_combined(color);
    let their_pawns = *board.pieces(Piece::Pawn) & *board.color_combined(enemy);

    let mut score: i32 = 0;

    // --- Pawn shield & open file penalties ---
    // Examine the king file and its neighbors (up to 3 files)
    let file_start = (king_file - 1).max(0);
    let file_end = (king_file + 1).min(7);

    for f in file_start..=file_end {
        let fmask = file_mask(File::from_index(f as usize));
        let friendly_on_file = our_pawns & fmask;
        let enemy_on_file = their_pawns & fmask;

        if friendly_on_file == EMPTY {
            // No friendly pawn on this file — king is exposed
            score -= 15;
            if enemy_on_file == EMPTY {
                // Fully open file next to king
                score -= 10;
            }
        } else {
            // Bonus for pawn shield proximity to king
            // Check rank 2 and 3 relative to the king's color
            let (shield_rank_1, shield_rank_2) = if color == Color::White {
                (Rank::from_index(1), Rank::from_index(2)) // ranks 2 and 3
            } else {
                (Rank::from_index(6), Rank::from_index(5)) // ranks 7 and 6
            };
            let r1_mask = BitBoard::set(shield_rank_1, File::from_index(f as usize));
            let r2_mask = BitBoard::set(shield_rank_2, File::from_index(f as usize));

            if friendly_on_file & r1_mask != EMPTY {
                score += 10; // pawn on home rank shielding king
            } else if friendly_on_file & r2_mask != EMPTY {
                score += 5; // pawn advanced one rank, still decent cover
            }
        }
    }

    // --- Enemy piece attacks into king zone ---
    let king_zone = chess::get_king_moves(king_sq) | BitBoard::from_square(king_sq);
    let occupied = *board.combined();

    // Knights
    let enemy_knights = *board.pieces(Piece::Knight) & *board.color_combined(enemy);
    for sq in enemy_knights {
        let attacks = chess::get_knight_moves(sq) & king_zone;
        if attacks != EMPTY {
            score -= 10;
        }
    }

    // Bishops
    let enemy_bishops = *board.pieces(Piece::Bishop) & *board.color_combined(enemy);
    for sq in enemy_bishops {
        let attacks = chess::get_bishop_moves(sq, occupied) & king_zone;
        if attacks != EMPTY {
            score -= 10;
        }
    }

    // Rooks
    let enemy_rooks = *board.pieces(Piece::Rook) & *board.color_combined(enemy);
    for sq in enemy_rooks {
        let attacks = chess::get_rook_moves(sq, occupied) & king_zone;
        if attacks != EMPTY {
            score -= 15;
        }
    }

    // Queens
    let enemy_queens = *board.pieces(Piece::Queen) & *board.color_combined(enemy);
    for sq in enemy_queens {
        let attacks = (chess::get_bishop_moves(sq, occupied) | chess::get_rook_moves(sq, occupied))
            & king_zone;
        if attacks != EMPTY {
            score -= 25;
        }
    }

    score
}

/// Calculate mobility (number of attacked squares) for a color
fn calculate_mobility(board: &Board, color: Color) -> i32 {
    // For mobility, we count the number of squares attacked by each piece
    // We use a temporary board with the given color to move to generate attacks

    let mut influence = 0;

    // Pawn attacks
    let pawns = *board.pieces(Piece::Pawn) & *board.color_combined(color);
    for sq in pawns {
        let attacks = chess::get_pawn_attacks(sq, color, *board.combined());
        influence += count_bits(attacks);
    }

    // Knight attacks
    let knights = *board.pieces(Piece::Knight) & *board.color_combined(color);
    for sq in knights {
        let attacks = chess::get_knight_moves(sq);
        influence += count_bits(attacks);
    }

    // Bishop attacks
    let bishops = *board.pieces(Piece::Bishop) & *board.color_combined(color);
    for sq in bishops {
        let attacks = chess::get_bishop_moves(sq, *board.combined());
        influence += count_bits(attacks);
    }

    // Rook attacks
    let rooks = *board.pieces(Piece::Rook) & *board.color_combined(color);
    for sq in rooks {
        let attacks = chess::get_rook_moves(sq, *board.combined());
        influence += count_bits(attacks);
    }

    // Queen attacks
    let queens = *board.pieces(Piece::Queen) & *board.color_combined(color);
    for sq in queens {
        let attacks = chess::get_bishop_moves(sq, *board.combined())
            | chess::get_rook_moves(sq, *board.combined());
        influence += count_bits(attacks);
    }

    // King attacks
    let king_sq = board.king_square(color);
    let king_attacks = chess::get_king_moves(king_sq);
    influence += count_bits(king_attacks);

    influence
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_starting_position_eval() {
        let board = Board::default();
        let score = eval(&board);
        // Starting position should be roughly equal
        assert!(
            score.abs() < 50.0,
            "Starting position eval {} should be near 0",
            score
        );
    }

    #[test]
    fn test_checkmate_eval() {
        // Scholar's mate position (Black is checkmated)
        let board =
            Board::from_str("r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4")
                .unwrap();
        let score = eval(&board);
        assert_eq!(score, MATE_EVAL);
    }
}
