/**
 * Texel Automated Tuning
 * This file is used to tune the evaluation weights of Vesper using a dataset of positions and their results.
 * The database is the 'lichess-big3-resolved'.
 * This file is currently not used in the main Vesper codebase, but can be used to generate a new set of weights for the evaluation function.
*/
use std::{
    f64::consts::LN_10, fs::File, io::{BufRead, BufReader}
};

use rayon::prelude::*;
use vesper::{
    attacks::Tables, bitboard::Color, board::Board, eval::{EvalMask, Weights, evaluate}, trace::{Trace, trace},
};

struct TuningPosition {
    board: Board,
    result: f64, // 1.0 = White won, 0.5 = Draw, 0.0 = Black won
    trace: Vec<f64>,
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

fn flatten_weights(w: &Weights) -> Vec<f64> {
    let mut v: Vec<f64> = Vec::new();
    v.extend(w.piece_values_mg.iter().map(|&x| x as f64));
    v.extend(w.piece_values_eg.iter().map(|&x| x as f64));

    for row in &w.pst_mg { v.extend(row.iter().map(|&x| x as f64)); }
    for row in &w.pst_eg { v.extend(row.iter().map(|&x| x as f64)); }

    v.push(w.doubled_pawn_mg as f64);
    v.push(w.doubled_pawn_eg as f64);

    v.push(w.isolated_pawn_mg as f64);
    v.push(w.isolated_pawn_eg as f64);

    v.extend(w.passed_pawn_bonus.iter().map(|&x| x as f64));

    v.push(w.bishop_pair_mg as f64);
    v.push(w.bishop_pair_eg as f64);

    v.extend(w.mobility_mg.iter().map(|&x| x as f64));
    v.extend(w.mobility_eg.iter().map(|&x| x as f64));

    v.extend(w.king_danger_table.iter().map(|&x| x as f64));

    v
}

fn unflatted_weights(w: &mut Weights, v: &[f64]) {
    let mut i: usize = 0;

    for x in w.piece_values_mg.iter_mut() { *x = v[i].round() as i32; i += 1; }
    for x in w.piece_values_eg.iter_mut() { *x = v[i].round() as i32; i += 1; }

    for row in w.pst_mg.iter_mut() { for x in row.iter_mut() { *x = v[i].round() as i32; i += 1; } }
    for row in w.pst_eg.iter_mut() { for x in row.iter_mut() { *x = v[i].round() as i32; i += 1; } }

    w.doubled_pawn_mg = v[i].round() as i32; i += 1;
    w.doubled_pawn_eg = v[i].round() as i32; i += 1;

    w.isolated_pawn_mg = v[i].round() as i32; i += 1;
    w.isolated_pawn_eg = v[i].round() as i32; i += 1;

    for x in w.passed_pawn_bonus.iter_mut() { *x = v[i].round() as i32; i += 1; }

    w.bishop_pair_mg = v[i].round() as i32; i += 1;
    w.bishop_pair_eg = v[i].round() as i32; i += 1;

    for x in w.mobility_mg.iter_mut() { *x = v[i].round() as i32; i += 1; }
    for x in w.mobility_eg.iter_mut() { *x = v[i].round() as i32; i += 1; }

    for x in w.king_danger_table.iter_mut() { *x = v[i].round() as i32; i += 1; }

    debug_assert_eq!(i, v.len(), "flatten/unflatten field order mismatch");
}

fn flatten_trace(trace: &Trace) -> Vec<f64> {
    let mut v: Vec<f64> = Vec::new();

    v.extend_from_slice(&trace.piece_values_mg);
    v.extend_from_slice(&trace.piece_values_eg);

    for row in &trace.pst_mg { v.extend_from_slice(row); }
    for row in &trace.pst_eg { v.extend_from_slice(row); }

    v.push(trace.doubled_pawn_mg);
    v.push(trace.doubled_pawn_eg);

    v.push(trace.isolated_pawn_mg);
    v.push(trace.isolated_pawn_eg);
    
    v.extend_from_slice(&trace.passed_pawn_bonus);
    
    v.push(trace.bishop_pair_mg);
    v.push(trace.bishop_pair_eg);
    
    v.extend_from_slice(&trace.mobility_mg);
    v.extend_from_slice(&trace.mobility_eg);
    
    v.extend_from_slice(&trace.king_danger_table);
    
    v
}

fn load_position(path: &str, tables: &Tables, masks: &EvalMask, weights: &Weights) -> Vec<TuningPosition> {
    let reader: BufReader<File> = BufReader::new(File::open(path).expect("dataset not found"));
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

    lines
        .par_iter()
        .filter_map(|line| {
            let (fen, result) = parse_line(line)?;
            let board = Board::from_fen(&fen).ok()?;
            let trace = flatten_trace(&trace(&board, tables, masks, weights));
            Some(TuningPosition { board, result, trace })
        })
        .collect()
}

fn sigmoid(score: f64, k: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-k * score / 400.0))
}

fn eval_from_trace(trace: &[f64], w: &[f64]) -> f64 {
    trace.iter().zip(w.iter()).map(|(t, wi)| t * wi).sum()
}

fn mean_squared_error(positions: &[TuningPosition], w: &[f64], k: f64) -> f64 {
    let total: f64 = positions
        .par_iter()
        .map(|pos: &TuningPosition| {
            let white_relative: f64 = eval_from_trace(&pos.trace, w);
            (pos.result - sigmoid(white_relative, k)).powi(2)
        })
        .sum();

    total / positions.len() as f64
}

// Computes mse with the real evaluate() function for the inner/outer rings
fn mean_squared_error_real_eval(positions: &[TuningPosition], tables: &Tables, masks: &EvalMask, weights: &Weights, k: f64) -> f64 {
    let total: f64 = positions
        .par_iter()
        .map(|pos: &TuningPosition| {
            let raw: f64 = evaluate(&pos.board, tables, masks, weights) as f64;
            let white_relative: f64 = if pos.board.side_to_move == Color::White { raw } else { -raw };
            (pos.result - sigmoid(white_relative, k)).powi(2)
        })
        .sum();

    total / positions.len() as f64
}

fn fit_k(positions: &[TuningPosition], w: &[f64]) -> f64 {
    let mut best_k: f64 = 1.0;
    let mut best_error: f64 = mean_squared_error(positions, w, best_k);

    for &step in &[0.1, 0.1, 0.01, 0.001] {
        loop {
            let mut improved: bool = false;

            for candidate in [best_k - step, best_k + step] {
                if candidate <= 0.0 {
                    continue;
                }

                let error = mean_squared_error(positions, w, candidate);

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

fn gradient(positions: &[TuningPosition], w: &[f64], k: f64) -> Vec<f64> {
    let param_count: usize = w.len();
    let ln10: f64 = LN_10;

    let summed: Vec<f64> = positions
        .par_iter()
        .fold(
            || vec![0.0; param_count],
            |mut grad, pos| {
                let raw = eval_from_trace(&pos.trace, w);
                let sig = sigmoid(raw, k);
                let coeff = 2.0 * (sig - pos.result) * sig * (1.0 - sig) * ln10 * k / 400.0;
                for (g, t) in grad.iter_mut().zip(pos.trace.iter()) {
                    *g += coeff * t;
                }
                grad
            }
        )
        .reduce(
            || vec![0.0; param_count],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(b.iter()) { *x += y; }
                a
            }
        );
    
    let n = positions.len() as f64;
    summed.into_iter().map(|g| g / n).collect()
}

fn clip_grad_norm(grad: &mut [f64], max_norm: f64) {
    let norm: f64 = grad.iter().map(|g| g * g).sum::<f64>().sqrt();

    if norm > max_norm && norm > 0.0 {
        let scale: f64 = max_norm / norm;
        for g in grad.iter_mut() { *g *= scale; }
    }
}

struct Adam {
    m: Vec<f64>,
    v: Vec<f64>,
    t: i32,
    lr: f64,
    beta1: f64,
    beta2: f64,
    eps: f64,
}

impl Adam {
    fn new(param_count: usize, lr: f64) -> Self {
        Adam {
            m: vec![0.0; param_count],
            v: vec![0.0; param_count],
            t: 0,
            lr,
            beta1: 0.9,
            beta2: 0.999,
            eps: 1E-8
        }
    }

    fn step(&mut self, w: &mut [f64], grad: &[f64], weight_decay: f64) {
        self.t += 1;
        let bc1: f64 = 1.0 - self.beta1.powi(self.t);
        let bc2: f64 = 1.0 - self.beta2.powi(self.t);

        for j in 0..w.len() {
            self.m[j] = self.beta1 * self.m[j] + (1.0 - self.beta1) * grad[j];
            self.v[j] = self.beta2 * self.v[j] + (1.0 - self.beta2) * grad[j] * grad[j];
            let m_hat = self.m[j] / bc1;
            let v_hat = self.v[j] / bc2;
            w[j] -= self.lr * (m_hat / (v_hat.sqrt() + self.eps) + weight_decay * w[j]);
        }
    }
}

fn tune(
    positions: &[TuningPosition],
    mut w: Vec<f64>,
    mut k: f64,
    epochs: usize,
    lr: f64,
    weight_decay: f64,
    refit_k_every: usize,
    max_grad_norm: f64,
) -> (Vec<f64>, f64) {
    let mut optimizer: Adam = Adam::new(w.len(), lr);
    let mut prev_error: f64 = mean_squared_error(positions, &w, k);
    println!("  starting error: {prev_error:.10}");

    for epoch in 0..epochs {
        let mut grad: Vec<f64> = gradient(positions, &w, k);
        clip_grad_norm(&mut grad, max_grad_norm);
        optimizer.step(&mut w, &grad, weight_decay);

        if refit_k_every > 0 && (epoch + 1) % refit_k_every == 0 {
            k = fit_k(positions, &w);
        }

        if epoch % 20 == 0 || epoch == epoch - 1 {
            let error: f64 = mean_squared_error(positions, &w, k);
            println!("  epoch {epoch}: error = {error:.10} (Δ {:.3e})", error - prev_error);
            prev_error = error;
        }
    }

    (w, k)
}

fn sweep_ring_weights(positions: &[TuningPosition], tables: &Tables, masks: &EvalMask, weights: &mut Weights, k: f64) {
    let mut total_error = mean_squared_error_real_eval(positions, tables, masks, weights, k);
    println!("  ring-weight sweep starting error: {total_error:.10}");

    for step in [4, 2, 1] {
        loop {
            let mut improved: bool = false;

            for idx in 1..=4usize {
                let mut best_delta: Option<i32> = None;
                let mut best_error: f64 = total_error;
                for delta in [step, -step] {
                    weights.inner_ring_weight[idx] += delta;
                    let err: f64 = mean_squared_error_real_eval(positions, tables, masks, weights, k);
                    weights.inner_ring_weight[idx] -= delta;
                    if err < best_error {
                        best_error = err;
                        best_delta = Some(delta);
                    }
                }
                if let Some(delta) = best_delta {
                    weights.inner_ring_weight[idx] += delta;
                    total_error = best_error;
                    improved = true;
                }
            }

            for idx in 1..=4usize {
                let mut best_delta: Option<i32> = None;
                let mut best_error: f64 = total_error;
                for delta in [step, -step] {
                    weights.outer_ring_weight[idx] += delta;
                    let err: f64 = mean_squared_error_real_eval(positions, tables, masks, weights, k);
                    weights.outer_ring_weight[idx] -= delta;
                    if err < best_error {
                        best_error = err;
                        best_delta = Some(delta);
                    }
                }
                if let Some(delta) = best_delta {
                    weights.outer_ring_weight[idx] += delta;
                    total_error = best_error;
                    improved = true;
                }
            }

            if !improved {
                break;
            }
        }
    }

    println!("  ring-weight sweep finished error: {total_error:.10}");
}

fn main() {
    let path: String = std::env::args().nth(1).expect("usage: tuner <dataset> [epochs_pre_round] [lr] [outer_rounds] [weight_decay]");
    let epochs_per_round: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let lr: f64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(0.05);
    let outer_rounds: usize = std::env::args().nth(4).and_then(|s| s.parse().ok()).unwrap_or(5);
    let weight_decay: f64 = std::env::args().nth(5).and_then(|s| s.parse().ok()).unwrap_or(1E-4);

    let tables: Tables = Tables::new();
    let masks: EvalMask = EvalMask::new();
    let mut weights: Weights = Weights::default();

    println!("loading positions and computing initial traces...");
    let mut positions = load_position(&path, &tables, &masks, &weights);
    println!("loaded {} positions, {} linear parameters (+12 ring weights tuned separately)", positions.len(), flatten_weights(&weights).len());

    let mut w: Vec<f64> = flatten_weights(&weights);
    let mut k: f64 = fit_k(&positions, &w);

    for round in 0..outer_rounds {
        println!("\n=== round {round}: gradient descent (linear weights) ===");
        let (new_w, new_k) = tune(&positions, w, k, epochs_per_round, lr, weight_decay, 50, 500.0);
        w = new_w;
        k = new_k;
        unflatted_weights(&mut weights, &w);

        println!("\n=== round {round}: coordinate descent (king-safety ring weights) ===");
        sweep_ring_weights(&positions, &tables, &masks, &mut weights, k);

        // Since ring weights changed, king_attack_points gets shifted
        // and it's necessary to re-compute the trace coefficients
        println!("recomputing traces after ring-weight update...");
        positions.par_iter_mut().for_each(|pos: &mut TuningPosition| {
            pos.trace = flatten_trace(&trace(&pos.board, &tables, &masks, &weights));
        });
        w = flatten_weights(&weights);
        k = fit_k(&positions, &w);
    }

    std::fs::write("tuned_weights.rs", print_weights_as_rust(&weights)).expect("failed to write tuned weights");
    println!("\nwrote tuned weights to tuned_weights.rs");
}

fn print_weights_as_rust(weights: &Weights) -> String {
    format!(
        "impl Default for Weights {{\n  fn default() -> Self {{\n       {:#?}\n     }}\n}}\n",
        weights
    )
}
