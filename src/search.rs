use std::{cmp::Reverse, time::Instant};

use crate::{attacks::Tables, bitboard::{Color, PieceType}, board::Board, eval::evaluate, moves::{Move, MoveFlag, generate_legal_moves, is_in_check}};

const MATE_VALUE: i32 = 30_000;
const INFINITY: i32 = 32_000;

const MVV_LVA_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 10_000]; // Pawn, Knight, Bishop, Rook, Queen, King

fn score_move(mv: Move, board: &Board) -> i32 {
    if !mv.flag().is_capture() { return 0; }
    let stm: Color = board.side_to_move;
    let opp: Color = stm.opposite();

    let victim: PieceType = if mv.flag() == MoveFlag::EnPassant {
        PieceType::Pawn
    } else {
        board.piece_on(opp, mv.to()).expect("Move has capture flag but no piece on to-square")
    };
    let attacker: PieceType = board.piece_on(stm, mv.from()).expect("No piece on from-square");

    10_000 + MVV_LVA_VALUES[victim as usize] * 10 - MVV_LVA_VALUES[attacker as usize]
}

fn order_moves(mut moves: Vec<Move>, board: &Board) -> Vec<Move> {
    moves.sort_by_key(|&mv| Reverse(score_move(mv, board)));
    moves
}

fn negamax(board: &mut Board, tables: &Tables, depth: u32, ply: u32, mut alpha: i32, beta: i32) -> i32 {
    if board.is_insufficient_material() { return 0; }
    if board.halfmove_clock >= 100 { return 0; }
    if depth == 0 { return quiescence(board, tables, ply, alpha, beta); }

    let moves: Vec<Move> = generate_legal_moves(board, tables);
    if moves.is_empty() {
        return if is_in_check(board, tables) {
            -(MATE_VALUE - ply as i32)
        } else {
            0
        }
    }

    for mv in order_moves(moves, board) {
        let undo = board.make_move(mv);
        let score = -negamax(board, tables, depth - 1, ply + 1, -beta, -alpha);
        board.unmake_move(mv, undo);

        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    
    alpha
}

fn quiescence(board: &mut Board, tables: &Tables, ply: u32, mut alpha: i32, beta: i32) -> i32 {
    let in_check: bool = is_in_check(board, tables);

    if !in_check {
        let stand_pat: i32 = evaluate(board);
        if stand_pat >= beta { return beta; }
        alpha = alpha.max(stand_pat);
    }

    let all_moves: Vec<Move> = generate_legal_moves(board, tables);
    if in_check && all_moves.is_empty() {
        return -(MATE_VALUE - ply as i32);
    }

    let candidates: Vec<Move> = if in_check {
        all_moves
    } else {
        all_moves.into_iter().filter(|mv| mv.flag().is_capture()).collect()
    };

    for mv in order_moves(candidates, board) {
        let undo = board.make_move(mv);
        let score = quiescence(board, tables, ply + 1, -beta, -alpha);
        board.unmake_move(mv, undo);

        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    
    alpha
}

/* Iterative Deepening */
pub fn search_best_move(board: &mut Board, tables: &Tables, deadline: Instant) -> Move {
    let root_moves: Vec<Move> = generate_legal_moves(board, tables);
    assert!(!root_moves.is_empty(), "search_best_move called with no legal moves available");

    let mut best_move = root_moves[0];

    for depth in 1.. {
        if Instant::now() >= deadline { break; }
        
        let mut alpha: i32 = -INFINITY;
        let mut depth_best_move: Move = root_moves[0];

        for mv in order_moves(root_moves.clone(), board) {
            let undo = board.make_move(mv);
            let score = -negamax(board, tables, depth - 1, 1, -INFINITY, -alpha);
            board.unmake_move(mv, undo);

            if score >= alpha {
                alpha = score;
                depth_best_move = mv;
            }
        }
        best_move = depth_best_move;
        if Instant::now() >= deadline { break; }
    }

    best_move
}