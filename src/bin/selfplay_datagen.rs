use std::{fs::File, io::{BufWriter, Write}, sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}}, time::Instant};

use vesper::{attacks::Tables, bitboard::Color, board::Board, eval::{EvalMask, EvalMode, Weights}, moves::{Move, generate_legal_moves, is_in_check}, search::{LMRTable, SearchControl, SearchInfo, is_repetition, search_best_move}, tt::TranspositionTable};
use rand::{Rng, RngExt};

const MIN_OPENING_PLIES: usize = 6;
const MAX_OPENING_PLIES: usize = 10;

const WIN_ADJUDICATION_CP: i32 = 1000;
const WIN_ADJUDICATION_PLIES: usize = 6;
const DRAW_ADJUDICATION_CP: i32 = 15;
const DRAW_ADJUDICATION_PLIES: usize = 10;

const NODE_LIMIT: u64 = 20_000;
const SAMPLE_PROBABILITY: f64 = 0.5;
const MAX_GAME_LEN: usize = 600;

const APPROX_MATE_THRESHOLD: i32 = 29_000;

struct Searcher {
    tables: Tables,
    tt: TranspositionTable,
    weights: Weights,
    lmr_table: LMRTable,
    eval_mask: EvalMask,
    eval_mode: EvalMode,
}

impl Searcher {
    fn new() -> Self {
        Self {
            tables: Tables::new(),
            tt: TranspositionTable::new(256),
            weights: Weights::default(),
            lmr_table: LMRTable::new(),
            eval_mask: EvalMask::new(),
            eval_mode: EvalMode::NNUE,
        }
    }

    fn search(&mut self, board: &mut Board, history: &mut Vec<u64>) -> (Move, Option<i32>) {
        let stop = Arc::new(AtomicBool::new(false));
        let global_nodes = Arc::new(AtomicU64::new(0));
        let deadline = Instant::now() + std::time::Duration::from_secs(3600);
        let mut control = SearchControl::with_node_limit(
            deadline,
            deadline,
            stop,
            global_nodes,
            Some(NODE_LIMIT),
        );

        let mut score: Option<i32> = None;
        let best_move = search_best_move(
            board,
            &self.tables,
            &self.tt,
            &self.weights,
            &self.lmr_table,
            &self.eval_mask,
            history,
            &mut control,
            None,
            0,
            &self.eval_mode,
            |info: &SearchInfo| score = Some(info.score),
        );

        (best_move, score)
    }
}

struct SampledPosition {
    fen: String,
    score_white_relative: i32,
}

fn play_game(rng: &mut impl Rng, searcher: &mut Searcher) -> (Vec<SampledPosition>, f64) {
    searcher.tt.clear();
    let mut board: Board = Board::start_position().unwrap();
    let mut history: Vec<u64> = vec![board.zobrist_key];
    let mut samples: Vec<SampledPosition> = Vec::new();

    let opening_plies = rng.random_range(MIN_OPENING_PLIES..=MAX_OPENING_PLIES);
    for _ in 0..opening_plies {
        let moves: Vec<Move> = generate_legal_moves(&board, &searcher.tables);
        if moves.is_empty() { break; }
        let mv: Move = moves[rng.random_range(0..moves.len())];
        let _ = board.make_move(mv);
        history.push(board.zobrist_key);
    }

    let mut consecutive_extreme: usize = 0usize;
    let mut consecutive_drawish: usize = 0usize;

    for _ in 0..MAX_GAME_LEN {
        let moves: Vec<Move> = generate_legal_moves(&board, &searcher.tables);
        if moves.is_empty() {
            let result = if is_in_check(&board, &searcher.tables) {
                if board.side_to_move == Color::White { 0.0 } else { 1.0 }
            } else {
                0.5
            };
            return (samples, result);
        }

        if board.halfmove_clock >= 100 || board.is_insufficient_material() || is_repetition(&board, &history){
            return (samples, 0.5);
        }

        let in_check_before: bool = is_in_check(&board, &searcher.tables);
        let (mv, score) = searcher.search(&mut board, &mut history);
        let Some(score) = score else {
            let _ = board.make_move(mv);
            history.push(board.zobrist_key);
            continue;
        };
        let white_relative: i32 = if board.side_to_move == Color::White { score } else { -score };

        if white_relative.abs() >= WIN_ADJUDICATION_CP {
            consecutive_extreme += 1;
            if consecutive_extreme >= WIN_ADJUDICATION_PLIES {
                return (samples, if white_relative > 0 { 1.0 } else { 0.0 });
            }
        } else {
            consecutive_extreme = 0;
        }

        if white_relative.abs() <= DRAW_ADJUDICATION_CP {
            consecutive_drawish += 1;
            if consecutive_drawish >= DRAW_ADJUDICATION_PLIES {
                return (samples, 0.5);
            }
        } else {
            consecutive_drawish = 0;
        }

        let quiet: bool = !in_check_before && !mv.flag().is_capture() && score.abs() < APPROX_MATE_THRESHOLD;

        if quiet && rng.random_bool(SAMPLE_PROBABILITY) {
            samples.push(SampledPosition {
                fen: board.to_fen(),
                score_white_relative: white_relative,
            });
        }

        let _ = board.make_move(mv);
        history.push(board.zobrist_key);
    }

    (samples, 0.5)
}

fn main() {
    let out_path: String = std::env::args().nth(1).unwrap_or_else(|| "selfplay_data.txt".to_string());
    let target_positions: u64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(10_000_000);
    let num_threads: usize = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(4) as usize;

    println!("generating up to {target_positions} positions across {num_threads} threads...");
    let written = AtomicU64::new(0);

    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for thread_id in 0..num_threads {
            let written = &written;
            let out_path = out_path.clone();
            handles.push(scope.spawn(move || {
                let mut rng = rand::rng();
                let mut out = BufWriter::new(
                    File::create(format!("{out_path}.part{thread_id}")).expect("failed to create shard"),
                );
                let mut searcher = Searcher::new();

                while written.load(Ordering::Relaxed) < target_positions {
                    let (samples, result) = play_game(&mut rng, &mut searcher);
                    for s in &samples {
                        writeln!(out, "{} | {} | {result}", s.fen, s.score_white_relative).expect("failed to write sample");
                    }
                    written.fetch_add(samples.len() as u64, Ordering::Relaxed);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    });

    println!("done — merge the .part* files, then run through the same bulletformat conversion as make_nnue_data.rs");
}