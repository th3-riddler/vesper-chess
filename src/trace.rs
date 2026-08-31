use crate::{attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::Board, eval::{EvalMask, KingEvalInfo, MAX_PHASE, PHASE_WEIGHTS, Weights, mirror_square}};

#[derive(Clone, Debug)]
pub struct Trace {
    pub piece_values_mg: [f64; 6],
    pub piece_values_eg: [f64; 6],
    pub pst_mg: [[f64; 64]; 6],
    pub pst_eg: [[f64; 64]; 6],
    pub doubled_pawn_mg: f64,
    pub doubled_pawn_eg: f64,
    pub isolated_pawn_mg: f64,
    pub isolated_pawn_eg: f64,
    pub passed_pawn_bonus: [f64; 8],
    pub bishop_pair_mg: f64,
    pub bishop_pair_eg: f64,
    pub mobility_mg: [f64; 6],
    pub mobility_eg: [f64; 6],

    /// Just for shape parity
    pub inner_ring_weight: [f64; 6],
    /// Just for shape parity
    pub outer_ring_weight: [f64; 6],

    pub king_danger_table: [f64; 64],
}

impl Default for Trace {
    fn default() -> Self {
        Trace {
            piece_values_mg: [0.0; 6],
            piece_values_eg: [0.0; 6],
            pst_mg: [[0.0; 64]; 6],
            pst_eg: [[0.0; 64]; 6],
            doubled_pawn_mg: 0.0,
            doubled_pawn_eg: 0.0,
            isolated_pawn_mg: 0.0,
            isolated_pawn_eg: 0.0,
            passed_pawn_bonus: [0.0; 8],
            bishop_pair_mg: 0.0,
            bishop_pair_eg: 0.0,
            mobility_mg: [0.0; 6],
            mobility_eg: [0.0; 6],
            inner_ring_weight: [0.0; 6],
            outer_ring_weight: [0.0; 6],
            king_danger_table: [0.0; 64]
        }
    }
}

fn trace_piece_position(board: &Board, trace: &mut Trace, phase: &mut i32, color: Color, sign: f64) {
    for piece in 0..6 {
        let mut p = board.pieces[color as usize][piece];
        while let Some(square) = p.pop_lsb() {
            let idx: usize = if color == Color::White { mirror_square(square) as usize } else { square as usize };

            trace.piece_values_mg[piece] += sign;
            trace.piece_values_eg[piece] += sign;
            trace.pst_mg[piece][idx] += sign;
            trace.pst_eg[piece][idx] += sign;

            *phase += PHASE_WEIGHTS[piece];
        }
    }
}

fn trace_pawn_structure(board: &Board, masks: &EvalMask, trace: &mut Trace, color: Color, sign: f64) {
    let stm_pawns: Bitboard = board.pieces[color as usize][PieceType::Pawn as usize];
    let opp_pawns: Bitboard = board.pieces[color.opposite() as usize][PieceType::Pawn as usize];

    for file in 0u8..8 {
        let count: i32 = (masks.get_file_mask(file) & stm_pawns).pop_count() as i32;
        if count > 1 {
            let extra: f64 = (count - 1) as f64;
            trace.doubled_pawn_mg += sign * extra;
            trace.doubled_pawn_eg += sign * extra;
        }
    }

    let mut pawns: Bitboard = stm_pawns;
    while let Some(square) = pawns.pop_lsb() {
        let file: u8 = square % 8;

        if masks.get_adjacent_file_mask(file) & stm_pawns == Bitboard::EMPTY {
            trace.isolated_pawn_mg += sign;
            trace.isolated_pawn_eg += sign;
        }

        if masks.get_passed_pawn_mask(square, color) & opp_pawns == Bitboard::EMPTY {
            let rank: u8 = square / 8;
            let relative_rank: u8 = if color == Color::White { rank } else { 7 - rank };

            trace.passed_pawn_bonus[relative_rank as usize] += sign;
        }
    }
}

fn trace_bishop_pair(board: &Board, trace: &mut Trace, color: Color, sign: f64) {
    let bishops: Bitboard = board.pieces[color as usize][PieceType::Bishop as usize];
    let has_light: bool = (bishops & Bitboard::LIGHT_SQUARES) != Bitboard::EMPTY;
    let has_dark: bool = (bishops & Bitboard::DARK_SQUARES) != Bitboard::EMPTY;

    if has_light && has_dark {
        trace.bishop_pair_mg += sign;
        trace.bishop_pair_eg += sign;
    }
}

fn trace_mobility(board: &Board, tables: &Tables, masks: &EvalMask, weights: &Weights, trace: &mut Trace, king_info: &mut KingEvalInfo, color: Color, sign: f64) {
    let own: Bitboard = board.occupancy(color);
    let occ: Bitboard = board.all_occupancy();
    let opp: Color = color.opposite();
    let opp_king_sq: u8 = board.pieces[opp as usize][PieceType::King as usize].0.trailing_zeros() as u8;

    for (piece, idx) in [
        (PieceType::Knight, 1),
        (PieceType::Bishop, 2),
        (PieceType::Rook, 3),
        (PieceType::Queen, 4),
    ] {
        let mut pieces: Bitboard = board.pieces[color as usize][piece as usize];
        while let Some(square) = pieces.pop_lsb() {
            let attacks: Bitboard = match piece {
                PieceType::Knight => tables.get_knight_attacks(square),
                PieceType::Bishop => tables.get_bishop_attacks(square, occ),
                PieceType::Rook => tables.get_rook_attacks(square, occ),
                PieceType::Queen => tables.get_queen_attacks(square, occ),
                _ => unreachable!()
            };

            let count: f64 = (attacks & !own).pop_count() as f64;
            trace.mobility_mg[idx] += sign * count;
            trace.mobility_eg[idx] += sign * count;

            // Still computed even if it's not part of the trace
            let inner: i32 = (attacks & !own & masks.get_king_inner_ring_mask(opp_king_sq)).pop_count() as i32;
            let outer: i32 = (attacks & !own & masks.get_king_outer_ring_mask(opp_king_sq)).pop_count() as i32;
            king_info.king_attack_points[opp as usize] += inner * weights.inner_ring_weight[idx] + outer * weights.outer_ring_weight[idx];
        }
    }
}

fn trace_king_safety(king_info: &KingEvalInfo, weights: &Weights, trace: &mut Trace, color: Color, sign: f64) {
    let max_index: i32 = (weights.king_danger_table.len() - 1) as i32;
    let index: usize = king_info.king_attack_points[color as usize].clamp(0, max_index) as usize;
    trace.king_danger_table[index] += sign;
}

fn scale_tapered_fields(trace: &mut Trace, mg_scale: f64, eg_scale: f64) {
    for v in trace.piece_values_mg.iter_mut() { *v *= mg_scale; }
    for v in trace.piece_values_eg.iter_mut() { *v *= eg_scale; }

    for row in trace.pst_mg.iter_mut() { for v in row.iter_mut() { *v *= mg_scale; } }
    for row in trace.pst_eg.iter_mut() { for v in row.iter_mut() { *v *= eg_scale; } }

    trace.doubled_pawn_mg *= mg_scale;
    trace.doubled_pawn_eg *= eg_scale;

    trace.isolated_pawn_mg *= mg_scale;
    trace.isolated_pawn_eg *= eg_scale;

    // trace.passed_pawn_bonus is untapered because there's no difference between mg and eg, so the scale would be 1

    trace.bishop_pair_mg *= mg_scale;
    trace.bishop_pair_eg *= eg_scale;

    for v in trace.mobility_mg.iter_mut() { *v *= mg_scale; }
    for v in trace.mobility_eg.iter_mut() { *v *= eg_scale; }
    
    // inner_ring_weight/outer_ring_weight are left at 0

    for v in trace.king_danger_table.iter_mut() { *v *= mg_scale; }
}

pub fn trace(board: &Board, tables: &Tables, masks: &EvalMask, weights: &Weights) -> Trace {
    let mut trace: Trace = Trace::default();
    let mut phase: i32 = 0;
    let mut king_info: KingEvalInfo =  KingEvalInfo::default();

    trace_piece_position(board, &mut trace, &mut phase, Color::White, 1.0);
    trace_piece_position(board, &mut trace, &mut phase, Color::Black, -1.0);

    trace_pawn_structure(board, masks, &mut trace, Color::White, 1.0);
    trace_pawn_structure(board, masks, &mut trace, Color::Black, -1.0);

    trace_bishop_pair(board, &mut trace, Color::White, 1.0);
    trace_bishop_pair(board, &mut trace, Color::Black, -1.0);

    trace_mobility(board, tables, masks, weights, &mut trace, &mut king_info, Color::White, 1.0);
    trace_mobility(board, tables, masks, weights, &mut trace, &mut king_info, Color::Black, -1.0);

    // Signs are opposite because the king danger table evaluates the danger to the king of the side to move
    trace_king_safety(&king_info, weights, &mut trace, Color::White, -1.0);
    trace_king_safety(&king_info, weights, &mut trace, Color::Black, 1.0);

    phase = phase.min(MAX_PHASE);
    let mg_scale: f64 = phase as f64 / MAX_PHASE as f64;
    let eg_scale: f64 = (MAX_PHASE - phase) as f64 / MAX_PHASE as f64;
    scale_tapered_fields(&mut trace, mg_scale, eg_scale);

    trace
}