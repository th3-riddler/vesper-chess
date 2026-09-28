# Vesper

**Vesper** is a chess engine written in Rust. It communicates through the Universal
Chess Interface ([UCI](https://chessprogramming.org/UCI)), so it can be used from a chess GUI or driven directly from a terminal.

The project is intended to be a compact, readable engine that can be improved
over time. Its implementation is organized around a bitboard chess position,
legal move generation, handcrafted/NNUE evaluation and a time-controlled game-tree
search.

## Features

Vesper currently provides:

- UCI input and output for integration with chess software
- [Bitboard](https://chessprogramming.org/Bitboards) board representation and precomputed attack tables
- Incremental make/unmake move support
- [Zobrist](https://chessprogramming.org/Zobrist_Hashing) position hashing and repetition tracking
- [Iterative deepening](https://chessprogramming.org/Iterative_Deepening) negamax search with alpha-beta pruning
- [Quiescence search](https://chessprogramming.org/Quiescence_Search) and capture [move ordering](https://chessprogramming.org/Move_Ordering)
- A [transposition table](https://chessprogramming.org/Transposition_Table) used during search
- A phase-aware material and piece-square evaluation
- A handcrafted evaluation function tuned using the [Texel Tuning's method](https://chessprogramming.org/Texel's_Tuning_Method)
- [Perft](https://chessprogramming.org/Perft) tests for validating move generation

## Building

Install a recent stable Rust toolchain using [rustup](https://rustup.rs/), then
build the engine with Cargo:

```text
cargo build --release
```

The optimized executable is produced at `target/release/vesper`.

## Running

Vesper is a UCI engine and reads commands from standard input. For example:

```text
./target/release/vesper
```

A minimal manual session looks like this:

```text
uci
isready
position startpos
go depth 12
quit
```

The engine also accepts a FEN position followed by a sequence of UCI moves:

```text
position fen <FEN> moves <move> <move> ...
```

The supported search controls are `depth`, `movetime`, `wtime`, `btime`,
`winc`, and `binc`. The standard `stop`, `ucinewgame`, and `quit` commands are
also supported. Unknown commands are ignored.

## Testing

Run the complete test suite with:

```text
cargo test --release
```

The tests cover move legality, special positions, incremental Zobrist hashing,
and known perft results for positions such as the initial position and
Kiwipete.

## Development Notes

Vesper is an evolving engine. Search strength, evaluation quality, protocol
coverage, and performance may change as development continues. The README
therefore focuses on stable concepts and workflows rather than benchmark
numbers or release-specific claims.
