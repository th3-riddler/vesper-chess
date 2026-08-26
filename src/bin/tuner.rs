/**
 * Texel Automated Tuning
 * This file is used to tune the evaluation weights of Vesper using a dataset of positions and their results.
 * The database is the 'lichess-big3-resolved'.
 * This file is currently not used in the main Vesper codebase, but can be used to generate a new set of weights for the evaluation function.
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

fn single_squared_error(
    pos: &TuningPosition,
    tables: &Tables,
    masks: &EvalMask,
    weights: &Weights,
    k: f64
) -> f64 {
    let raw = evaluate(&pos.board, tables, masks, weights) as f64;
    let white_relative = if pos.board.side_to_move == Color::White {
        raw
    } else {
        -raw
    };
    (pos.result - sigmoid(white_relative, k)).powi(2)
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
        .map(|pos: &TuningPosition| { single_squared_error(pos, tables, masks, weights, k)})
        .sum();

    total / positions.len() as f64
}

fn evaluate_errors(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    weights: &Weights,
    k: f64,
) -> (Vec<f64>, f64) {
    let errors: Vec<f64> = positions
        .par_iter()
        .map(|pos| single_squared_error(pos, tables, masks, weights, k))
        .collect();

    let mse = errors.iter().sum::<f64>() / positions.len() as f64;

    (errors, mse)
}

fn fit_k(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    weights: &Weights,
) -> f64 {
    let mut best_k = 1.0;
    let mut best_error =
        mean_squared_error(positions, tables, masks, weights, best_k);

    for &step in &[0.1, 0.01, 0.001] {
        loop {
            let mut improved = false;

            for candidate in [best_k - step, best_k + step] {
                if candidate <= 0.0 {
                    continue;
                }

                let error =
                    mean_squared_error(positions, tables, masks, weights, candidate);

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

    println!("fitted k = {best_k:.6}, error = {best_error:.10}");

    best_k
}

fn tune(
    positions: &[TuningPosition],
    tables: &Tables,
    masks: &EvalMask,
    mut weights: Weights,
    k: f64,
) -> Weights {
    let param_count = weights.iter_mut().count();

    let (_, mut total_error) =
        evaluate_errors(positions, tables, masks, &weights, k);

    println!("starting error: {total_error:.10}");

    for step in [16, 8, 4, 2, 1] {
        loop {
            let mut improved = false;

            for i in 0..param_count {
                let mut best_delta: Option<i32> = None;
                let mut best_error = total_error;

                for delta in [step, -step] {
                    let mut candidate = weights.clone();
                    *candidate.iter_mut().nth(i).unwrap() += delta;

                    let (_, candidate_error) =
                        evaluate_errors(
                            positions,
                            tables,
                            masks,
                            &candidate,
                            k,
                        );

                    if candidate_error < best_error {
                        best_error = candidate_error;
                        best_delta = Some(delta);
                    }
                }

                if let Some(delta) = best_delta {
                    *weights.iter_mut().nth(i).unwrap() += delta;
                    total_error = best_error;
                    improved = true;

                    println!(
                        "step {step}: parameter {i} += {delta}, \terror = {total_error}"
                    );
                }
            }

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

    let tables = Tables::new();
    let masks = EvalMask::new();

    let mut weights = Weights::default();

    for iteration in 0..10 {
        println!("\n=== Tuning iteration {iteration} ===");

        let k = fit_k(
            &positions,
            &tables,
            &masks,
            &weights,
        );

        weights = tune(
            &positions,
            &tables,
            &masks,
            weights,
            k,
        );
    }

    std::fs::write(
        "tuned_weight.rs",
        print_weights_as_rust(&weights),
    )
    .expect("failed to write tuned weights");

    println!("wrote tuned weights to tuned_weight.rs");
}

fn print_weights_as_rust(weights: &Weights) -> String {
    format!(
        "impl Default for Weights {{\n  fn default() -> Self {{\n       {:#?}\n     }}\n}}\n",
        weights
    )
}
