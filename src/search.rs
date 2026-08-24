/*  =======================
         Move ordering
    =======================
    
    1. PV move
    2. SEE
    3. Queen Promotions
    4. 1st killer move
    5. 2nd killer move
    6. History moves
    7. Unsorted moves
*/

use std::{
    cmp::{Reverse, min}, sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    }, time::Instant,
};

use crate::{
    attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::{Board, NullMoveUndo}, eval::{EvalMask, evaluate}, moves::{Move, UndoInfo, generate_legal_moves, is_in_check}, see::see, tt::{Bound, TTEntry, TranspositionTable},
};

pub(crate) const MATE_VALUE: i32 = 30_000;
pub(crate) const MATE_THRESHOLD: i32 = MATE_VALUE - 1000;
const INFINITY: i32 = 32_000;

const MAX_PLY: usize = 128;

const NULL_MOVE_MIN_DEPTH: u32 = 3;

pub struct SearchControl {
    deadline: Instant,
    stop: Arc<AtomicBool>,
    nodes: u64,
    aborted: bool,
    killers: [[Move; 2]; MAX_PLY],
    history_score: [[[i32; 64]; 64]; 2],
}

impl SearchControl {
    pub fn new(deadline: Instant, stop: Arc<AtomicBool>) -> Self {
        SearchControl {
            deadline,
            stop,
            nodes: 0,
            aborted: false,
            killers: [[Move::NULL; 2]; MAX_PLY],
            history_score: [[[0; 64]; 64]; 2],
        }
    }

    fn record_killer(&mut self, ply: u32, mv: Move) {
        let ply: usize = ply as usize;
        if ply >= MAX_PLY { return; }
        
        if self.killers[ply][0] != mv {
            self.killers[ply][1] = self.killers[ply][0];
            self.killers[ply][0] = mv;
        }
    }

    fn record_history(&mut self, stm: Color, mv: Move, depth: u32) {
        self.history_score[stm as usize][mv.from() as usize][mv.to() as usize] += (depth * depth) as i32;
    }

    pub fn decay_history(&mut self) {
        for color_table in self.history_score.iter_mut() {
            for row in color_table.iter_mut() {
                for cell in row.iter_mut() {
                    *cell /= 2;
                }
            }
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

pub struct LMRTable {
    table: [[u32; 64]; 64],
}

impl LMRTable {
    pub fn new() -> Self {
        let mut lmr_table: [[u32; 64]; 64] = [[0; 64]; 64];
        lmr_table[0][0] = 0;
        for depth in 1..64 {
            for move_index in 1..64 {
                lmr_table[depth][move_index] = (0.75 + (depth as f64).ln() * (move_index as f64).ln() / 2.25) as u32;
            }
        }
        LMRTable { table: lmr_table }
    }
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

const HASH_MOVE_SCORE: i32 = 2_000_000;
const GOOD_CAPTURE_BASE: i32 = 1_500_000;
const QUEEN_PROMOTION_SCORE: i32 = 1_400_000;
const KILLER_1_SCORE: i32 = 1_300_000;
const KILLER_2_SCORE: i32 = 1_299_000;
const BAD_CAPUTRE_BASE: i32 = -1_000_000;

const LMR_MIN_DEPTH: u32 = 3;
const LMR_MIN_MOVE_INDEX: usize = 3;

// After this depth, disable Futility Pruning
const FUTILITY_MAX_DEPTH: u32 = 8;
const FUTILITY_MARGIN_PER_DEPTH: i32 = 100;

const RAZOR_MAX_DEPTH: u32 = 3;

const MAX_EXTENSIONS_PER_PATH: u32 = 8;

fn score_move(mv: Move, board: &Board, tables: &Tables, ctrl: &mut SearchControl, ply: u32, hash_move: Option<Move>) -> i32 {
    if Some(mv) == hash_move { return HASH_MOVE_SCORE; }
    let stm: Color = board.side_to_move;

    if mv.flag().is_capture() {
        let value: i32 = see(board, tables, mv);
        return if value >= 0 { GOOD_CAPTURE_BASE + value } else { BAD_CAPUTRE_BASE + value };
    }

    if mv.flag().promotion_piece() == Some(PieceType::Queen) {
        return QUEEN_PROMOTION_SCORE;
    }

    let ply_idx: usize = ply as usize;
    if ply_idx < MAX_PLY {
        if mv == ctrl.killers[ply_idx][0] { return KILLER_1_SCORE; }
        if mv == ctrl.killers[ply_idx][1] { return KILLER_2_SCORE; }
    }

    ctrl.history_score[stm as usize][mv.from() as usize][mv.to() as usize]
}

fn order_moves(mut moves: Vec<Move>, board: &Board, tables: &Tables, ctrl: &mut SearchControl, ply: u32, hash_move: Option<Move>) -> Vec<Move> {
    moves.sort_by_key(|&mv| Reverse(score_move(mv, board, tables, ctrl, ply, hash_move)));
    moves
}

fn has_non_pawn_material(board: &Board, stm: Color) -> bool {
    [PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .any(|&pt| board.pieces[stm as usize][pt as usize] != Bitboard::EMPTY)
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

fn null_move_reduction(depth: u32) -> u32 {
    3 + depth / 6
}

fn razor_margin(depth: u32) -> i32 {
    200 + 150 * depth as i32
}

fn negamax(
    board: &mut Board,
    tables: &Tables,
    tt: &mut TranspositionTable,
    lmr_table: &LMRTable,
    eval_mask: &EvalMask,
    history: &mut Vec<u64>,
    ctrl: &mut SearchControl,
    depth: u32,
    ply: u32,
    extensions_used: u32,
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

    let pv_node: bool = beta - alpha > 1; 

    let tt_entry: Option<TTEntry> = tt.probe(board.zobrist_key);
    if let Some(entry) = tt_entry
        && entry.depth() as u32 >= depth
    {
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

    if depth == 0 {
        return quiescence(board, tables, tt, ctrl, ply, eval_mask, alpha, beta);
    }

    let in_check: bool = is_in_check(board, tables);
    
    // Null Move Pruning
    if !in_check
        && ply > 0
        && !pv_node
        && depth >= NULL_MOVE_MIN_DEPTH
        && beta < MATE_THRESHOLD
        && has_non_pawn_material(board, board.side_to_move)
    {
        let undo: NullMoveUndo = board.make_null_move();
        history.push(board.zobrist_key);

        // Dynamic Null Move Pruning
        let reduction: u32 = null_move_reduction(depth);
        let reduced_depth: u32 = depth.saturating_sub(1 + reduction);
        let score: i32 = -negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, reduced_depth, ply + 1, extensions_used, -beta, -beta + 1);
        
        history.pop();
        board.unmake_null_move(undo);

        if ctrl.is_aborted() { return alpha; }
        if score >= beta {
            return beta;
        }
    }

    let static_eval: i32 = evaluate(board, tables, eval_mask);

    // Razoring
    if !in_check && !pv_node && depth <= RAZOR_MAX_DEPTH && beta < MATE_THRESHOLD {
        let margin: i32 = razor_margin(depth);
        if static_eval + margin <= alpha {
            let razor_score: i32 = quiescence(board, tables, tt, ctrl, ply, eval_mask, alpha, beta);
            if razor_score <= alpha {
                return razor_score;
            }
        }
    }
    
    // Futility Pruning Node-Level
    if !in_check && !pv_node && depth <= FUTILITY_MAX_DEPTH && beta < MATE_THRESHOLD {
        let margin: i32 = FUTILITY_MARGIN_PER_DEPTH * depth as i32;
        if static_eval - margin >= beta {
            return static_eval - margin;
        }
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

    for (i, mv) in order_moves(moves, board, tables, ctrl, ply, hash_move).into_iter().enumerate() {
        let is_quiet: bool = !mv.flag().is_capture() && mv.flag().promotion_piece().is_none();

        // Futility Pruning Move-Level
        if is_quiet
            && !in_check
            && !pv_node
            && depth <= FUTILITY_MAX_DEPTH
            && i > 0
            && static_eval + FUTILITY_MARGIN_PER_DEPTH * depth as i32 <= alpha
            && best_score > -MATE_THRESHOLD
        {
            continue;
        }

        let undo: UndoInfo = board.make_move(mv);
        let gives_check: bool = is_in_check(board, tables);
        history.push(board.zobrist_key);

        let mut extension: u32 = if gives_check { 1 } else { 0 };
        if extension > 0 && extensions_used >= MAX_EXTENSIONS_PER_PATH {
            extension = 0;
        }
        let child_depth: u32 = (depth - 1 + extension).min(depth);
        let child_extensions: u32 = extensions_used + extension;

        // PVS with Null Window
        let score: i32 = if i == 0 {
            -negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, child_depth, ply + 1, child_extensions, -beta, -alpha)
        } else {
            // Late Move Reduction
            let reduction: u32 = if is_quiet
                && !pv_node
                && !in_check
                && !gives_check
                && depth >= LMR_MIN_DEPTH
                && i >= LMR_MIN_MOVE_INDEX
            {
                let d: usize = min(depth, 63) as usize;
                let m: usize = min(i, 63);
                lmr_table.table[d][m]
            } else {
                0
            };
            let reduced_depth: u32 = depth.saturating_sub(1 + reduction);
            let mut probe: i32 = -negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, reduced_depth, ply + 1, child_extensions, -alpha - 1, -alpha);

            if reduction > 0 && probe > alpha {
                probe = -negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, child_depth, ply + 1, child_extensions, -alpha - 1, -alpha);
            }

            if probe > alpha && probe < beta {
                -negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, child_depth, ply + 1, child_extensions, -beta, -alpha)
            } else {
                probe
            }
        };
        
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
            if !mv.flag().is_capture() {
                ctrl.record_killer(ply, mv);
                ctrl.record_history(board.side_to_move, mv, depth);
            }
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
    eval_mask: &EvalMask,
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
        best_score = evaluate(board, tables, eval_mask);
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

    const QSEARCH_SEE_MARGIN: i32 = -100;

    // Currently, moves are ordered by SEE >= QSEARCH_SEE_MARGIN, filtering out only unambiguously bad captures.
    let candidates: Vec<Move> = if in_check {
        all_moves
    } else {
        let mut captures: Vec<(Move, i32)> = all_moves
            .into_iter()
            .filter(|mv: &Move| mv.flag().is_capture())
            .map(|mv: Move| (mv, see(board, tables, mv)))
            .filter(|&(_, score)| score >= QSEARCH_SEE_MARGIN)
            .collect();
        
        captures.sort_by_key(|&(_, score)| Reverse(score));
        captures.into_iter().map(|(mv, _)| mv).collect()
    };

    for mv in candidates {
        let undo: UndoInfo = board.make_move(mv);
        let score: i32 = -quiescence(board, tables, tt, ctrl, ply + 1, eval_mask, -beta, -alpha);
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
    lmr_table: &LMRTable,
    eval_mask: &EvalMask,
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

    let mut score: i32 = 0;
    for depth in 1..=max_depth.unwrap_or(u32::MAX) {
        if ctrl.stop.load(Ordering::Relaxed) || Instant::now() >= ctrl.deadline {
            break;
        }

        // Aspiration Windows
        let mut window: i32 = 50;
        let (mut alpha, mut beta) = if depth <= 2 {
            (-INFINITY, INFINITY)
        } else {
            (score - window, score + window)
        };

        loop {
            score = negamax(board, tables, tt, lmr_table, eval_mask, history, ctrl, depth, 0, 0, alpha, beta);
            if ctrl.is_aborted() { break; }

            if score <= alpha {
                alpha = (alpha - window).max(-INFINITY);
                window *= 4;
            } else if score >= beta {
                beta = (beta + window).min(INFINITY);
                window *= 4;
            } else {
                break;
            }
        }

        if ctrl.is_aborted() { break; }

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