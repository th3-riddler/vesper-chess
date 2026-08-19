use std::{
    cmp::Reverse,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use crate::{
    attacks::Tables,
    bitboard::{Color, PieceType},
    board::Board,
    eval::evaluate,
    moves::{Move, MoveFlag, UndoInfo, generate_legal_moves, is_in_check},
    tt::{Bound, TTEntry, TranspositionTable},
};

pub(crate) const MATE_VALUE: i32 = 30_000;
const INFINITY: i32 = 32_000;

pub(crate) const MATE_THRESHOLD: i32 = MATE_VALUE - 1000;

const MVV_LVA_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 10_000]; // Pawn, Knight, Bishop, Rook, Queen, King

pub struct SearchControl {
    deadline: Instant,
    stop: Arc<AtomicBool>,
    nodes: u64,
    aborted: bool,
}

impl SearchControl {
    pub fn new(deadline: Instant, stop: Arc<AtomicBool>) -> Self {
        SearchControl {
            deadline,
            stop,
            nodes: 0,
            aborted: false,
        }
    }

    fn poll(&mut self) -> bool {
        self.nodes += 1;
        if !self.aborted
            && self.nodes % 2048 == 0
            && (self.stop.load(Ordering::Relaxed) || Instant::now() >= self.deadline)
        {
            self.aborted = true;
        }
        self.aborted
    }

    pub fn is_aborted(&self) -> bool {
        self.aborted
    }

    fn count_node(&mut self) {
        self.nodes += 1;
    }
}

pub struct SearcInfo {
    pub depth: u32,
    pub score: i32,
    pub nodes: u64,
    pub time_ms: u128,
    pub pv: Vec<Move>,
}

fn score_to_tt(score: i32, ply: u32) -> i32 {
    if score >= MATE_THRESHOLD {
        score + ply as i32
    } else if score <= -MATE_THRESHOLD {
        score - ply as i32
    } else {
        score
    }
}

fn score_from_tt(score: i32, ply: u32) -> i32 {
    if score >= MATE_THRESHOLD {
        score - ply as i32
    } else if score <= -MATE_THRESHOLD {
        score + ply as i32
    } else {
        score
    }
}

fn score_move(mv: Move, board: &Board) -> i32 {
    if !mv.flag().is_capture() {
        return 0;
    }
    let stm: Color = board.side_to_move;
    let opp: Color = stm.opposite();

    let victim: PieceType = if mv.flag() == MoveFlag::EnPassant {
        PieceType::Pawn
    } else {
        board
            .piece_on(opp, mv.to())
            .expect("Move has capture flag but no piece on to-square")
    };
    let attacker: PieceType = board
        .piece_on(stm, mv.from())
        .expect("No piece on from-square");

    10_000 + MVV_LVA_VALUES[victim as usize] * 10 - MVV_LVA_VALUES[attacker as usize]
}

fn order_moves(mut moves: Vec<Move>, board: &Board, hash_move: Option<Move>) -> Vec<Move> {
    moves.sort_by_key(|&mv| {
        if Some(mv) == hash_move {
            Reverse(i32::MAX)
        } else {
            Reverse(score_move(mv, board))
        }
    });
    moves
}

fn is_repetition(board: &Board, history: &[u64]) -> bool {
    let limit: usize = board.halfmove_clock as usize;
    history
        .iter()
        .rev()
        .skip(1)
        .take(limit)
        .any(|&k| k == board.zobrist_key)
}

fn negamax(
    board: &mut Board,
    tables: &Tables,
    tt: &mut TranspositionTable,
    history: &mut Vec<u64>,
    ctrl: &mut SearchControl,
    depth: u32,
    ply: u32,
    mut alpha: i32,
    beta: i32,
) -> i32 {
    if ctrl.poll() {
        return alpha;
    }

    if ply > 0
        && (board.is_insufficient_material()
            || board.halfmove_clock >= 100
            || is_repetition(board, history))
    {
        return 0;
    }

    let tt_entry: Option<TTEntry> = tt.probe(board.zobrist_key);
    if let Some(entry) = tt_entry
        && entry.depth() as u32 >= depth
    {
        let score: i32 = score_from_tt(entry.score(), ply);
        let cutoff = match entry.bound() {
            Bound::Exact => true,
            Bound::Lower => score >= beta,
            Bound::Upper => score <= alpha,
        };
        if cutoff {
            return score;
        }
    }

    if depth == 0 {
        return quiescence(board, tables, tt, ctrl, ply, alpha, beta);
    }

    let moves: Vec<Move> = generate_legal_moves(board, tables);
    if moves.is_empty() {
        return if is_in_check(board, tables) {
            -(MATE_VALUE - ply as i32)
        } else {
            0
        };
    }

    let alpha_orig: i32 = alpha;
    let hash_move: Option<Move> = tt_entry.map(|e: TTEntry| e.best_move());
    let mut best_move: Move = moves[0];
    let mut best_score: i32 = -INFINITY;

    for mv in order_moves(moves, board, hash_move) {
        let undo: UndoInfo = board.make_move(mv);
        history.push(board.zobrist_key);

        let score: i32 = -negamax(
            board,
            tables,
            tt,
            history,
            ctrl,
            depth - 1,
            ply + 1,
            -beta,
            -alpha,
        );
        history.pop();
        board.unmake_move(mv, undo);

        if ctrl.is_aborted() {
            return best_score.max(alpha_orig);
        }

        if score > best_score {
            best_score = score;
            best_move = mv;
        }
        alpha = alpha.max(best_score);
        if alpha >= beta {
            break;
        }
    }

    let bound: Bound = if best_score <= alpha_orig {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    tt.store(
        board.zobrist_key,
        depth as u8,
        score_to_tt(best_score, ply),
        best_move,
        bound,
    );

    best_score
}

fn quiescence(
    board: &mut Board,
    tables: &Tables,
    tt: &mut TranspositionTable,
    ctrl: &mut SearchControl,
    ply: u32,
    mut alpha: i32,
    beta: i32,
) -> i32 {
    ctrl.count_node();
    if ctrl.is_aborted() {
        return alpha;
    }

    let alpha_orig: i32 = alpha;
    let tt_entry: Option<TTEntry> = tt.probe(board.zobrist_key);
    if let Some(entry) = tt_entry {
        let score: i32 = score_from_tt(entry.score(), ply);
        let cutoff: bool = match entry.bound() {
            Bound::Exact => true,
            Bound::Lower => score >= beta,
            Bound::Upper => score <= alpha,
        };
        if cutoff {
            return score;
        }
    }

    let in_check: bool = is_in_check(board, tables);
    let mut best_score;
    let mut best_move: Move = Move::NULL;

    if !in_check {
        best_score = evaluate(board);
        if best_score >= beta {
            tt.store(
                board.zobrist_key,
                0,
                score_to_tt(best_score, ply),
                Move::NULL,
                Bound::Lower,
            );
            return best_score;
        }
        alpha = alpha.max(best_score);
    } else {
        best_score = -INFINITY;
    }

    let all_moves: Vec<Move> = generate_legal_moves(board, tables);
    if in_check && all_moves.is_empty() {
        let score: i32 = -(MATE_VALUE - ply as i32);
        tt.store(
            board.zobrist_key,
            0,
            score_to_tt(score, ply),
            Move::NULL,
            Bound::Exact,
        );
        return score;
    }

    let candidates: Vec<Move> = if in_check {
        all_moves
    } else {
        all_moves
            .into_iter()
            .filter(|mv: &Move| mv.flag().is_capture())
            .collect()
    };

    for mv in order_moves(candidates, board, tt_entry.map(|e: TTEntry| e.best_move())) {
        let undo: UndoInfo = board.make_move(mv);
        let score: i32 = -quiescence(board, tables, tt, ctrl, ply + 1, -beta, -alpha);
        board.unmake_move(mv, undo);

        if score > best_score {
            best_score = score;
            best_move = mv;
        }
        if best_score >= beta {
            break;
        }
        alpha = alpha.max(best_score);
    }

    let bound: Bound = if best_score <= alpha_orig {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };

    tt.store(
        board.zobrist_key,
        0,
        score_to_tt(best_score, ply),
        best_move,
        bound,
    );

    best_score
}

fn extract_pv(
    board: &mut Board,
    tables: &Tables,
    tt: &mut TranspositionTable,
    max_len: u32,
) -> Vec<Move> {
    let mut pv: Vec<Move> = Vec::new();
    let mut undo_stack: Vec<UndoInfo> = Vec::new();
    let mut seen: Vec<u64> = Vec::new();

    for _ in 0..max_len {
        let Some(entry) = tt.probe(board.zobrist_key) else {
            break;
        };
        if entry.best_move() == Move::NULL {
            break;
        }
        let legal: Vec<Move> = generate_legal_moves(board, tables);
        if !legal.contains(&entry.best_move()) {
            break;
        }
        if seen.contains(&board.zobrist_key) {
            break;
        }

        seen.push(board.zobrist_key);
        pv.push(entry.best_move());
        undo_stack.push(board.make_move(entry.best_move()));
    }

    for &mv in pv.iter().rev() {
        board.unmake_move(mv, undo_stack.pop().unwrap());
    }

    pv
}

/* Iterative Deepening */
pub fn search_best_move(
    board: &mut Board,
    tables: &Tables,
    tt: &mut TranspositionTable,
    history: &mut Vec<u64>,
    ctrl: &mut SearchControl,
    max_depth: Option<u32>,
    mut on_info: impl FnMut(&SearcInfo),
) -> Move {
    let root_moves: Vec<Move> = generate_legal_moves(board, tables);
    assert!(
        !root_moves.is_empty(),
        "search_best_move called with no legal moves available"
    );

    let mut best_move: Move = root_moves[0];
    let start: Instant = Instant::now();

    for depth in 1..=max_depth.unwrap_or(u32::MAX) {
        if ctrl.stop.load(Ordering::Relaxed) || Instant::now() >= ctrl.deadline {
            break;
        }
        let score: i32 = negamax(
            board, tables, tt, history, ctrl, depth, 0, -INFINITY, INFINITY,
        );
        if ctrl.is_aborted() {
            break;
        }

        if let Some(entry) = tt.probe(board.zobrist_key) {
            best_move = entry.best_move();
        }
        let pv: Vec<Move> = extract_pv(board, tables, tt, depth);
        on_info(&SearcInfo {
            depth,
            score,
            nodes: ctrl.nodes,
            time_ms: start.elapsed().as_millis(),
            pv,
        })
    }

    best_move
}