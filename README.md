# XewaliChessRust

A Rust port of the [XewaliChess](https://github.com/hsaikia/XewaliChess) engine, originally written in C++. Unlike the original, which uses move generation code from the [Stockfish repository](https://github.com/daylen/stockfish-mac/tree/master/Chess), this port uses the [`chess`](https://crates.io/crates/chess) crate for move generation and board representation.

## Links

- [Original C++ Repository](https://github.com/hsaikia/XewaliChess)
- [YouTube - Algorithm Explanation](https://www.youtube.com/watch?v=E7FGXCbwImI)
- [Lichess Bot (xewali)](https://lichess.org/@/xewali)

## Algorithms

### Search

- **Iterative Deepening** - Progressively searches at increasing depths (1, 2, 3, ...) until the time limit is reached. This provides an anytime search capability and improves move ordering across iterations.
- **Minimax with Alpha-Beta Pruning** - The core search algorithm. Alpha-beta pruning eliminates branches that cannot influence the final decision, reducing the effective branching factor from O(b^d) toward O(b^(d/2)).
- **Quiescence Search** - At leaf nodes (depth 0), the engine continues searching capture sequences to avoid the horizon effect. Implements recapture logic where only moves to the same square are considered after a capture.
- **Transposition Table** - A hash map keyed by Zobrist hash stores previously evaluated positions. Cached evaluations are reused when the stored depth is sufficient, avoiding redundant computation.
- **Move Ordering** - Child moves are sorted by their evaluation score to maximize alpha-beta cutoffs.

### Evaluation

The static evaluation function combines three components:

- **Material Balance** - Standard piece values (Pawn: 100, Knight: 320, Bishop: 330, Rook: 500, Queen: 900).
- **Piece-Square Tables** - Each piece type has a positional bonus table that encourages good piece placement (e.g., central knights, 7th-rank rooks). The king uses separate middlegame and endgame tables, switching based on remaining material.
- **Mobility** - Counts the number of squares influenced by each side's pieces. The mobility bonus is calculated as `10 * ln(white_influence / black_influence)`.

### Opening Book

The engine can load an opening book from a UCI game file. When the current position is found in the book, the engine randomly selects from known book moves instead of searching.

## Project Structure

```
src/
├── main.rs          UCI protocol interface and entry point
├── engine.rs        Search (iterative deepening, minimax, alpha-beta, quiescence)
├── evaluation.rs    Static evaluation (material, piece-square tables, mobility)
└── book.rs          Opening book loading and lookup
```

## Building

```bash
cargo build --release
```

The binary is built as `xewali_engine` with `opt-level = 3` and LTO enabled.

## Usage

The engine communicates via the [UCI protocol](https://en.wikipedia.org/wiki/Universal_Chess_Interface) and can be used with any UCI-compatible chess GUI.

```
$ ./target/release/xewali_engine
uci
id name Xewali 1.0
id author Himangshu Saikia
uciok
```

## License

MIT
