use std::{cmp::Reverse, time::Instant};

use crate::{attacks::Tables, bitboard::{Color, PieceType}, board::Board, eval::evaluate, moves::{Move, MoveFlag, UndoInfo, generate_legal_moves, is_in_check}, tt::{Bound, TTEntry, TranspositionTable}};

const MATE_VALUE: i32 = 30_000;
const INFINITY: i32 = 32_000;

const MATE_THRESHOLD: i32 = MATE_VALUE - 1000;

const MVV_LVA_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 10_000]; // Pawn, Knight, Bishop, Rook, Queen, King

fn score_to_tt(score: i32, ply: u32) -> i32 {
    if score >= MATE_THRESHOLD { score + ply as i32 }
    else if score <= - MATE_THRESHOLD { score - ply as i32 }
    else { score }
}

fn score_from_tt(score: i32, ply: u32) -> i32 {
    if score >= MATE_THRESHOLD { score - ply as i32 }
    else if score <= -MATE_THRESHOLD { score + ply as i32 }
    else { score }
}

fn tt_best_move_at_root(board: &Board, tt: &mut TranspositionTable) -> Option<Move> {
    tt.probe(board.zobrist_key).map(|e| e.best_move())
}

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

fn order_moves(mut moves: Vec<Move>, board: &Board, hash_move: Option<Move>) -> Vec<Move> {
    moves.sort_by_key(|&mv| {
        if Some(mv) == hash_move { Reverse(i32::MAX) } else { Reverse(score_move(mv, board)) }
    });
    moves
}

fn is_repetition(board: &Board, history: &[u64]) -> bool {
    let limit: usize = board.halfmove_clock as usize;
    history.iter().rev().take(limit).any(|&k| k == board.zobrist_key)
}

fn negamax(
    board: &mut Board, tables: &Tables, tt: &mut TranspositionTable,
    history: &mut Vec<u64>, depth: u32, ply: u32, mut alpha: i32, beta: i32
) -> i32 {
    
    if ply > 0 && (board.is_insufficient_material() || board.halfmove_clock >= 100 || is_repetition(board, history)) {
        return 0;
    }

    let tt_entry: Option<TTEntry> = tt.probe(board.zobrist_key);
    if let Some(entry) = tt_entry {
        if entry.depth() as u32 >= depth {
            let score: i32 = score_from_tt(entry.score(), ply);
            let cutoff = match entry.bound() {
                Bound::Exact => true,
                Bound::Lower => score >= beta,
                Bound::Upper => score <= alpha
            };
            if cutoff { return score; }
        }
    }

    if depth == 0 { return quiescence(board, tables, tt_entry, ply, alpha, beta); }

    let moves: Vec<Move> = generate_legal_moves(board, tables);
    if moves.is_empty() {
        return if is_in_check(board, tables) { -(MATE_VALUE - ply as i32) } else { 0 }
    }

    let alpha_orig: i32 = alpha;
    let hash_move: Option<Move> = tt_entry.map(|e| e.best_move());
    let mut best_move: Move = moves[0];
    let mut best_score = -INFINITY;

    for mv in order_moves(moves, board, hash_move) {
        let undo: UndoInfo = board.make_move(mv);
        history.push(board.zobrist_key);

        let score: i32 = -negamax(board, tables, tt, history, depth - 1, ply + 1, -beta, -alpha);
        history.pop();
        board.unmake_move(mv, undo);

        if score > best_score { best_score = score; best_move = mv; }
        alpha = alpha.max(best_score);
        if alpha >= beta { break; }
    }
    
    let bound: Bound = if best_score <= alpha_orig { Bound::Upper }
                       else if best_score >= beta { Bound::Lower }
                       else { Bound::Exact };
    tt.store(board.zobrist_key, depth as u8, score_to_tt(best_score, ply), best_move, bound);

    best_score
}

fn quiescence(board: &mut Board, tables: &Tables, tt_entry: Option<TTEntry>, ply: u32, mut alpha: i32, beta: i32) -> i32 {
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

    let hash_move: Option<Move> = tt_entry.map(|e| e.best_move());
    for mv in order_moves(candidates, board, hash_move) {
        let undo = board.make_move(mv);
        let score = quiescence(board, tables, tt_entry, ply + 1, -beta, -alpha);
        board.unmake_move(mv, undo);

        if score >= beta { return beta; }
        alpha = alpha.max(score);
    }
    
    alpha
}

/* Iterative Deepening */
pub fn search_best_move(board: &mut Board, tables: &Tables, tt: &mut TranspositionTable, history: &mut Vec<u64>, deadline: Instant) -> Move {
    let root_moves: Vec<Move> = generate_legal_moves(board, tables);
    assert!(!root_moves.is_empty(), "search_best_move called with no legal moves available");

    let mut best_move: Move = root_moves[0];

    for depth in 1.. {
        if Instant::now() >= deadline { break; }
        
        negamax(board, tables, tt, history, depth, 0, -INFINITY, INFINITY);
        if let Some(entry_move) = tt_best_move_at_root(board, tt) { best_move = entry_move }

        if Instant::now() >= deadline { break; }
    }

    best_move
}