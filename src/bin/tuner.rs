/**
 * Texel Automated Tuning
 * This file is used to tune the evaluation weights of Vesper using a dataset of positions and their results.
 * The database is the 'lichess-big3-resolved
*/
use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use rayon::prelude::*;
use vesper::{
    attacks::Tables,
    bitboard::Color,
    board::Board,
    eval::{evaluate, EvalMask, Weights},
};

struct TuningPosition {
    board: Board,
    result: f64, // 1.0 = White won, 0.5 = Draw, 0.0 = Black won
}

fn parse_line(line: &str) -> Option<(String, f64)> {
    let line: &str = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some(bracket_start) = line.rfind('[') {
        let fen: String = line[..bracket_start].trim().to_string();
        let result_str: &str = line[bracket_start + 1..].trim_end_matches(']').trim();
        let result: f64 = result_str.parse().ok()?;
        return Some((fen, result));
    }

    None
}

fn load_position(path: &str) -> Vec<TuningPosition> {
    let reader: BufReader<File> = BufReader::new(File::open(path).expect("dataset not found"));
    let mut positions: Vec<TuningPosition> = Vec::new();

    for line in reader.lines() {
        let line: String = line.unwrap();
        let Some((fen, result)) = parse_line(&line) else {
            continue;
        };

        if let Ok(board) = Board::from_fen(&fen) {
            positions.push(TuningPosition { board, result });
        }
    }

    positions
}

fn sigmoid(score: f64, k: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-k * score / 400.0))
}

fn mean_squared_error(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    weights: &Weights,
    k: f64,
) -> f64 {
    let total: f64 = positions
        .par_iter()
        .map(|pos: &TuningPosition| {
            let raw = evaluate(&pos.board, tables, masks, weights) as f64;
            let white_relative: f64 = if pos.board.side_to_move == Color::White {
                raw
            } else {
                -raw
            };
            (pos.result - sigmoid(white_relative, k)).powi(2)
        })
        .sum();

    total / positions.len() as f64
}

fn fit_k(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    weights: &Weights,
) -> f64 {
    let mut best_k: f64 = 1.0;
    let mut best_error: f64 = mean_squared_error(positions, tables, masks, weights, best_k);
    for &step in &[0.1, 0.01, 0.001] {
        loop {
            let mut improved: bool = false;
            for candidate in [best_k - step, best_k + step] {
                let error: f64 = mean_squared_error(positions, tables, masks, weights, candidate);
                if error < best_error {
                    best_error = error;
                    best_k = candidate;
                    improved = true;
                }
            }
            if !improved {
                break;
            }
        }
    }
    println!("fitted k = {best_k:.4}, error = {best_error:.6}");

    best_k
}

fn tune(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    mut weights: Weights,
    k: f64,
) -> Weights {
    let param_count: usize = weights.iter_mut().count();
    let mut best_error: f64 = mean_squared_error(positions, tables, masks, &weights, k);
    println!("starting error: {best_error:.6}");

    for step in [16, 8, 4, 2, 1] {
        loop {
            let mut improved: bool = false;
            for i in 0..param_count {
                for delta in [step, -step] {
                    let mut candidate = weights.clone();
                    *candidate.iter_mut().nth(i).unwrap() += delta;
                    let error: f64 = mean_squared_error(positions, tables, masks, &candidate, k);
                    if error < best_error {
                        best_error = error;
                        weights = candidate;
                        improved = true;
                    }
                }
            }
            println!("step {step}: error = {best_error:.6}");
            if !improved {
                break;
            }
        }
    }

    weights
}

fn main() {
    let positions = load_position(&std::env::args().nth(1).unwrap());
    println!("loaded {} positions", positions.len());

    let tables: Tables = Tables::new();
    let masks: EvalMask = EvalMask::new();
    let weights: Weights = Weights::default();

    let k: f64 = fit_k(&positions, &tables, &masks, &weights);
    let tuned: Weights = tune(&positions, &tables, &masks, weights, k);

    std::fs::write("tuned_weight.rs", print_weights_as_rust(&tuned))
        .expect("failed to write tuned_weights.rs");

    println!("wrote tuned weights to tuned_weights.rs");
}

fn print_weights_as_rust(weights: &Weights) -> String {
    format!(
        "impl Default for Weights {{\n  fn default() -> Self {{\n       {:#?}\n     }}\n}}\n",
        weights
    )
}
