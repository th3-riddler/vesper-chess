use std::{ops::{Add, AddAssign, Sub}};

use crate::{attacks::{Tables, mask_king_attacks}, bitboard::{Bitboard, Color, PieceType}, board::Board};

#[derive(Clone, Copy, Default)]
struct Score {
    mg: i32,
    eg: i32,
}

impl Add for Score {
    type Output = Score;
    fn add(self, rhs: Self) -> Self::Output {
        Score {
            mg: self.mg + rhs.mg,
            eg: self.eg + rhs.eg
        }
    }
}

impl Sub for Score {
    type Output = Score;
    fn sub(self, rhs: Self) -> Self::Output {
        Score {
            mg: self.mg - rhs.mg,
            eg: self.eg - rhs.eg
        }
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

pub struct EvalMask {
    file: [Bitboard; 8],
    adjacent_files: [Bitboard; 8],
    passed_pawn: [[Bitboard; 64]; 2],
    king_outer_ring: [Bitboard; 64],
    king_inner_ring: [Bitboard; 64]
}

impl EvalMask {
    pub fn new() -> Self {
        let mut file: [Bitboard; 8] = [Bitboard::EMPTY; 8];
        let mut adjacent_file: [Bitboard; 8] = [Bitboard::EMPTY; 8];
        let mut passed_pawn: [[Bitboard; 64]; 2] = [[Bitboard::EMPTY; 64]; 2];
        let mut king_outer_ring: [Bitboard; 64] = [Bitboard::EMPTY; 64];
        let mut king_inner_ring: [Bitboard; 64] = [Bitboard::EMPTY; 64];

        for f in 0u8..8 { file[f as usize] = mask_file_occ(f); }
        for f in 0u8..8 {
            adjacent_file[f as usize] = if f == 0 {
                file[1]
            } else if f == 7 {
                file[6]
            } else {
                file[(f - 1) as usize] | file[(f + 1) as usize]
            };
        }

        for square in 0u8..64 {
            passed_pawn[Color::White as usize][square as usize] = mask_passed_pawn(square, Color::White);
            passed_pawn[Color::Black as usize][square as usize] = mask_passed_pawn(square, Color::Black);
            king_outer_ring[square as usize] = mask_king_outer_ring(square);
            king_inner_ring[square as usize] = mask_king_attacks(square);
        }

        EvalMask {
            file,
            adjacent_files: adjacent_file,
            passed_pawn,
            king_outer_ring,
            king_inner_ring,
        }
    }

    pub fn get_file_mask(&self, file: u8) -> Bitboard {
        self.file[file as usize]
    }
    pub fn get_adjacent_file_mask(&self, file: u8) -> Bitboard {
        self.adjacent_files[file as usize]
    }
    pub fn get_passed_pawn_mask(&self, square: u8, color: Color) -> Bitboard {
        self.passed_pawn[color as usize][square as usize]
    }
    pub fn get_king_outer_ring_mask(&self, square: u8) -> Bitboard {
        self.king_outer_ring[square as usize]
    }
    pub fn get_king_inner_ring_mask(&self, square: u8) -> Bitboard {
        self.king_inner_ring[square as usize]
    }
}

fn mask_king_outer_ring(square: u8) -> Bitboard {
    let mut mask: Bitboard = Bitboard::EMPTY;
    let rank: u8 = square / 8;
    let file: u8 = square % 8;

    for r in rank.saturating_sub(2)..=(rank + 2).min(7) {
        for f in file.saturating_sub(2)..=(file + 2).min(7) {
            if (r == rank && f == file) || (r >= rank.saturating_sub(1) && r <= (rank + 1).min(7) && f >= file.saturating_sub(1) && f <= (file + 1).min(7)) {
                continue;
            }
            let sq: u8 = r * 8 + f;
            mask.0 |= 1u64 << sq;
        }
    }
    mask
}

fn mask_file_occ(file: u8) -> Bitboard {
    let mut mask: Bitboard = Bitboard::EMPTY;
    for rank in 0u8..8 {
        let square: u8 = rank * 8 + file;
        mask.0 |= 1u64 << square;
    }
    mask
}

#[inline]
fn mask_passed_pawn(square: u8, color: Color) -> Bitboard {
    let mut mask: Bitboard = Bitboard::EMPTY;
    let rank: u8 = square / 8;
    let file: u8 = square % 8;

    let ranks: Vec<u8> = if color == Color::White { ((rank + 1)..8).collect() } else { (0..rank).rev().collect() };
    for r in ranks {
        mask.0 |= 1u64 << (r * 8 + file);
        if file > 0 {
            mask.0 |= 1u64 << (r * 8 + (file - 1));
        }
        if file < 7 {
            mask.0 |= 1u64 << (r * 8 + (file + 1));
        }
    }
    mask
}

#[derive(Clone, Copy, Default)]
struct KingEvalInfo {
    // Stores the number of attack points for each color's king
    // The attack points are calculated based on the number of squares of the king's inner and outer rings attacked by a piece, weighted by the piece type
    king_attack_points: [i32; 2],
}

const fn build_king_danger_table() -> [i32; 64] {
    let mut table: [i32; 64] = [0; 64];
    
    let mut i: usize = 0;
    while i < 64 {
        let points: i32 = i as i32;
        table[i] = KING_DANGER_SCALE_NUM * points * points / KING_DANGER_SCALE_DEN;
        i += 1;
    }

    table
}

#[derive(Clone, Debug)]
pub struct Weights {
    pub piece_values_mg: [i32; 6],
    pub piece_values_eg: [i32; 6],
    pub pst_mg: [[i32; 64]; 6],
    pub pst_eg: [[i32; 64]; 6],
    pub doubled_pawn_mg: i32,
    pub doubled_pawn_eg: i32,
    pub isolated_pawn_mg: i32,
    pub isolated_pawn_eg: i32,
    pub passed_pawn_bonus: [i32; 8],
    pub bishop_pair_mg: i32,
    pub bishop_pair_eg: i32,
    pub mobility_mg: [i32; 6],
    pub mobility_eg: [i32; 6],
    pub inner_ring_weight: [i32; 6],
    pub outer_ring_weight: [i32; 6],
    pub king_danger_table: [i32; 64],
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            piece_values_mg: PIECE_VALUES_MG,
            piece_values_eg: PIECE_VALUES_EG,
            pst_mg: PESTO_MG,
            pst_eg: PESTO_EG,
            doubled_pawn_mg: DOUBLED_PAWN_PENALTY_MG,
            doubled_pawn_eg: DOUBLED_PAWN_PENALTY_EG,
            isolated_pawn_mg: ISOLATED_PAWN_PENALTY_MG,
            isolated_pawn_eg: ISOLATED_PAWN_PENALTY_EG,
            passed_pawn_bonus: PASSED_PAWN_BONUS,
            bishop_pair_mg: BISHOP_PAIR_BONUS_MG,
            bishop_pair_eg: BISHOP_PAIR_BONUS_EG,
            mobility_mg: MOBILITY_WEIGHT_MG,
            mobility_eg: MOBILITY_WEIGHT_EG,
            inner_ring_weight: INNER_RING_WEIGHT,
            outer_ring_weight: OUTER_RING_WEIGHT,
            king_danger_table: KING_DANGER_TABLE
        }
    }
}

impl Weights {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut i32> {
        self.piece_values_mg.iter_mut()
            .chain(self.piece_values_eg.iter_mut())
            .chain(self.pst_mg.iter_mut().flatten())
            .chain(self.pst_eg.iter_mut().flatten())
            .chain(std::iter::once(&mut self.doubled_pawn_mg))
            .chain(std::iter::once(&mut self.doubled_pawn_eg))
            .chain(std::iter::once(&mut self.isolated_pawn_mg))
            .chain(std::iter::once(&mut self.isolated_pawn_eg))
            .chain(self.passed_pawn_bonus.iter_mut())
            .chain(std::iter::once(&mut self.bishop_pair_mg))
            .chain(std::iter::once(&mut self.bishop_pair_eg))
            .chain(self.mobility_mg.iter_mut())
            .chain(self.mobility_eg.iter_mut())
            .chain(self.inner_ring_weight.iter_mut())
            .chain(self.outer_ring_weight.iter_mut())
            .chain(self.king_danger_table.iter_mut())
    }
}

// --------------------------------------------
// Material
// --------------------------------------------

const PIECE_VALUES_MG: [i32; 6] = [82, 337, 365, 477, 1025, 0]; // Pawn, Knight, Bishop, Rook, Queen, King
const PIECE_VALUES_EG: [i32; 6] = [94, 281, 297, 512, 936, 0];

// --------------------------------------------
// Piece-Square Tables
// --------------------------------------------

// PeSTO's Middlegame tables
#[rustfmt::skip]
const PESTO_MG: [[i32; 64]; 6] = [
    // Pawn
    [ 0,   0,   0,   0,   0,   0,  0,   0,
     98, 134,  61,  95,  68, 126, 34, -11,
     -6,   7,  26,  31,  65,  56, 25, -20,
    -14,  13,   6,  21,  23,  12, 17, -23,
    -27,  -2,  -5,  12,  17,   6, 10, -25,
    -26,  -4,  -4, -10,   3,   3, 33, -12,
    -35,  -1, -20, -23, -15,  24, 38, -22,
      0,   0,   0,   0,   0,   0,  0,   0],
    
    // Knight
    [-167, -89, -34, -49,  61, -97, -15, -107,
     -73, -41,  72,  36,  23,  62,   7,  -17,
     -47,  60,  37,  65,  84, 129,  73,   44,
      -9,  17,  19,  53,  37,  69,  18,   22,
     -13,   4,  16,  13,  28,  19,  21,   -8,
     -23,  -9,  12,  10,  19,  17,  25,  -16,
     -29, -53, -12,  -3,  -1,  18, -14,  -19,
    -105, -21, -58, -33, -17, -28, -19,  -23],

    // Bishop
    [-29,   4, -82, -37, -25, -42,   7,  -8,
    -26,  16, -18, -13,  30,  59,  18, -47,
    -16,  37,  43,  40,  35,  50,  37,  -2,
     -4,   5,  19,  50,  37,  37,   7,  -2,
     -6,  13,  13,  26,  34,  12,  10,   4,
      0,  15,  15,  15,  14,  27,  18,  10,
      4,  15,  16,   0,   7,  21,  33,   1,
    -33,  -3, -14, -21, -13, -12, -39, -21],
    
    // Rook
    [32,  42,  32,  51, 63,  9,  31,  43,
     27,  32,  58,  62, 80, 67,  26,  44,
     -5,  19,  26,  36, 17, 45,  61,  16,
    -24, -11,   7,  26, 24, 35,  -8, -20,
    -36, -26, -12,  -1,  9, -7,   6, -23,
    -45, -25, -16, -17,  3,  0,  -5, -33,
    -44, -16, -20,  -9, -1, 11,  -6, -71,
    -19, -13,   1,  17, 16,  7, -37, -26],
    
    // Queen
    [-28,   0,  29,  12,  59,  44,  43,  45,
    -24, -39,  -5,   1, -16,  57,  28,  54,
    -13, -17,   7,   8,  29,  56,  47,  57,
    -27, -27, -16, -16,  -1,  17,  -2,   1,
     -9, -26,  -9, -10,  -2,  -4,   3,  -3,
    -14,   2, -11,  -2,  -5,   2,  14,   5,
    -35,  -8,  11,   2,   8,  15,  -3,   1,
     -1, -18,  -9,  10, -15, -25, -31, -50],
    
    // King
    [-65,  23,  16, -15, -56, -34,   2,  13,
     29,  -1, -20,  -7,  -8,  -4, -38, -29,
     -9,  24,   2, -16, -20,   6,  22, -22,
    -17, -20, -12, -27, -30, -25, -14, -36,
    -49,  -1, -27, -39, -46, -44, -33, -51,
    -14, -14, -22, -46, -44, -30, -15, -27,
      1,   7,  -8, -64, -43, -16,   9,   8,
    -15,  36,  12, -54,   8, -28,  24,  14]
];

// PeSTO's Endgame tables
#[rustfmt::skip]
const PESTO_EG: [[i32; 64]; 6] = [
    // Pawn
    [0,   0,   0,   0,   0,   0,   0,   0,
    178, 173, 158, 134, 147, 132, 165, 187,
     94, 100,  85,  67,  56,  53,  82,  84,
     32,  24,  13,   5,  -2,   4,  17,  17,
     13,   9,  -3,  -7,  -7,  -8,   3,  -1,
      4,   7,  -6,   1,   0,  -5,  -1,  -8,
     13,   8,   8,  10,  13,   0,   2,  -7,
      0,   0,   0,   0,   0,   0,   0,   0],
    
    // Knight
    [-58, -38, -13, -28, -31, -27, -63, -99,
    -25,  -8, -25,  -2,  -9, -25, -24, -52,
    -24, -20,  10,   9,  -1,  -9, -19, -41,
    -17,   3,  22,  22,  22,  11,   8, -18,
    -18,  -6,  16,  25,  16,  17,   4, -18,
    -23,  -3,  -1,  15,  10,  -3, -20, -22,
    -42, -20, -10,  -5,  -2, -20, -23, -44,
    -29, -51, -23, -15, -22, -18, -50, -64],

    // Bishop
    [-14, -21, -11,  -8, -7,  -9, -17, -24,
     -8,  -4,   7, -12, -3, -13,  -4, -14,
      2,  -8,   0,  -1, -2,   6,   0,   4,
     -3,   9,  12,   9, 14,  10,   3,   2,
     -6,   3,  13,  19,  7,  10,  -3,  -9,
    -12,  -3,   8,  10, 13,   3,  -7, -15,
    -14, -18,  -7,  -1,  4,  -9, -15, -27,
    -23,  -9, -23,  -5, -9, -16,  -5, -17],
    
    // Rook
    [13, 10, 18, 15, 12,  12,   8,   5,
    11, 13, 13, 11, -3,   3,   8,   3,
     7,  7,  7,  5,  4,  -3,  -5,  -3,
     4,  3, 13,  1,  2,   1,  -1,   2,
     3,  5,  8,  4, -5,  -6,  -8, -11,
    -4,  0, -5, -1, -7, -12,  -8, -16,
    -6, -6,  0,  2, -9,  -9, -11,  -3,
    -9,  2,  3, -1, -5, -13,   4, -20],
    
    // Queen
    [-9,  22,  22,  27,  27,  19,  10,  20,
    -17,  20,  32,  41,  58,  25,  30,   0,
    -20,   6,   9,  49,  47,  35,  19,   9,
      3,  22,  24,  45,  57,  40,  57,  36,
    -18,  28,  19,  47,  31,  34,  39,  23,
    -16, -27,  15,   6,   9,  17,  10,   5,
    -22, -23, -30, -16, -16, -23, -36, -32,
    -33, -28, -22, -43,  -5, -32, -20, -41],
    
    // King
    [-74, -35, -18, -18, -11,  15,   4, -17,
    -12,  17,  14,  17,  17,  38,  23,  11,
     10,  17,  23,  15,  20,  45,  44,  13,
     -8,  22,  24,  27,  26,  33,  26,   3,
    -18,  -4,  21,  24,  27,  23,   9, -11,
    -19,  -3,  11,  21,  23,  16,   7,  -9,
    -27, -11,   4,  13,  14,   4,  -5, -17,
    -53, -34, -21, -11, -28, -14, -24, -43]
];

// --------------------------------------------
// Pawn Structure
// --------------------------------------------

const DOUBLED_PAWN_PENALTY_MG: i32 = -10;
const DOUBLED_PAWN_PENALTY_EG: i32 = -20;

const ISOLATED_PAWN_PENALTY_MG: i32 = -12;
const ISOLATED_PAWN_PENALTY_EG: i32 = -8;

const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 35, 50, 70, 0];

// --------------------------------------------
// Bishop Pair
// --------------------------------------------

const BISHOP_PAIR_BONUS_MG: i32 = 30;
const BISHOP_PAIR_BONUS_EG: i32 = 45;

// --------------------------------------------
// Mobility
// --------------------------------------------

const MOBILITY_WEIGHT_MG: [i32; 6] = [0, 4, 5, 3, 2, 0]; // Pawn, Knight, Bishop, Rook, Queen, King
const MOBILITY_WEIGHT_EG: [i32; 6] = [0, 3, 4, 4, 3, 0];

// --------------------------------------------
// King Safety
// --------------------------------------------

const OUTER_RING_WEIGHT: [i32; 6] = [0, 1, 1, 2, 3, 0];
const INNER_RING_WEIGHT: [i32; 6] = [0, 2, 2, 4, 5, 0];

const KING_DANGER_SCALE_NUM: i32 = 1;
const KING_DANGER_SCALE_DEN: i32 = 8;
const KING_DANGER_TABLE: [i32; 64] = build_king_danger_table();

// --------------------------------------------
// Phase
// --------------------------------------------

const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

#[inline]
fn mirror_square(square: u8) -> u8 { square ^ 56 }

fn evaluate_piece_position(board: &Board, weights: &Weights, phase: &mut i32, color: Color) -> Score {
    let (mut mg, mut eg) = (0, 0);

    for piece in 0..6 {
        let mut p: Bitboard = board.pieces[color as usize][piece];
        while let Some(square) = p.pop_lsb() {
            let idx: usize = if color == Color::White { mirror_square(square) as usize } else { square as usize };

            mg += weights.piece_values_mg[piece] + weights.pst_mg[piece][idx];
            eg += weights.piece_values_eg[piece] + weights.pst_eg[piece][idx];
            *phase += PHASE_WEIGHTS[piece];
        }
    }

    Score { mg, eg }
}

fn evaluate_pawn_structure(board: &Board, weights: &Weights, masks: &EvalMask, color: Color) -> Score {
    let (mut mg, mut eg) = (0, 0);
    let stm_pawns: Bitboard = board.pieces[color as usize][PieceType::Pawn as usize];
    let opp_pawns: Bitboard = board.pieces[color.opposite() as usize][PieceType::Pawn as usize];

    // Doubled Pawns
    for f in 0u8..8 {
        let count: u32 = (stm_pawns & masks.get_file_mask(f)).pop_count();
        if count > 1 {
            let extra: i32 = (count - 1) as i32;
            
            mg += weights.doubled_pawn_mg * extra;
            eg += weights.doubled_pawn_eg * extra;
        }
    }

    let mut pawns: Bitboard = stm_pawns;
    while let Some(square) = pawns.pop_lsb() {
        let file: u8 = square % 8;

        // Isolated Pawns
        if stm_pawns & masks.get_adjacent_file_mask(file) == Bitboard::EMPTY {
            mg += weights.isolated_pawn_mg;
            eg += weights.isolated_pawn_eg;
        }
        
        // Passed Pawns
        if masks.get_passed_pawn_mask(square, color) & opp_pawns == Bitboard::EMPTY {
            let rank: u8 = square / 8;
            let relative_rank = if color == Color::White { rank } else { 7 - rank };

            mg += weights.passed_pawn_bonus[relative_rank as usize];
            eg += weights.passed_pawn_bonus[relative_rank as usize];
        }
    }    

    Score { mg, eg }
}

fn evaluate_bishop_pair(board: &Board, weights: &Weights, color: Color) -> Score {
    let bishops: Bitboard = board.pieces[color as usize][PieceType::Bishop as usize];
    let has_light: bool = (bishops & Bitboard::LIGHT_SQUARES) != Bitboard::EMPTY;
    let has_dark: bool = (bishops & Bitboard::DARK_SQUARES) != Bitboard::EMPTY;

    if has_light && has_dark {
        Score {
            mg: weights.bishop_pair_mg,
            eg: weights.bishop_pair_eg
        }
    } else {
        Score::default()
    }
}

fn evaluate_mobility(
    board: &Board, tables: &Tables, masks: &EvalMask, weights: &Weights,
    king_info: &mut KingEvalInfo, color: Color
) -> Score {
    let (mut mg, mut eg) = (0, 0);
    let own: Bitboard = board.occupancy(color);
    let occ: Bitboard = board.all_occupancy();
    let opp: Color = color.opposite();

    let opp_king_sq: u8 = board.pieces[opp as usize][PieceType::King as usize].0.trailing_zeros() as u8;

    for (piece, idx) in [
        (PieceType::Knight, 1),
        (PieceType::Bishop, 2),
        (PieceType::Rook, 3),
        (PieceType::Queen, 4)
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
            let count: i32 = (attacks & !own).pop_count() as i32;
            mg += weights.mobility_mg[idx] * count;
            eg += weights.mobility_eg[idx] * count;

            let inner: i32 = (attacks & !own & masks.get_king_inner_ring_mask(opp_king_sq)).pop_count() as i32;
            let outer: i32 = (attacks & !own & masks.get_king_outer_ring_mask(opp_king_sq)).pop_count() as i32;

            king_info.king_attack_points[opp as usize] += inner * weights.inner_ring_weight[idx] + outer * weights.outer_ring_weight[idx];
        }
    }

    Score { mg, eg }
}

fn evaluate_king_attacks(points: i32, weights: &Weights) -> Score {
    let max_index: i32 = (weights.king_danger_table.len() - 1) as i32;
    let index: usize = points.clamp(0, max_index) as usize;

    Score {
        mg: -weights.king_danger_table[index],
        eg: 0,
    }
}

fn evaluate_king_safety(king_info: &KingEvalInfo, weights: &Weights, color: Color) -> Score {
    let mut score: Score = Score::default();
    
    score += evaluate_king_attacks(king_info.king_attack_points[color as usize], weights);

    score
}

pub fn evaluate(board: &Board, tables: &Tables, masks: &EvalMask, weights: &Weights) -> i32 {
    let mut score: Score = Score::default();
    let mut phase: i32 = 0;
    let mut king_info: KingEvalInfo = KingEvalInfo::default();

    score += evaluate_piece_position(board, weights, &mut phase, Color::White) - evaluate_piece_position(board, weights, &mut phase, Color::Black);
    score += evaluate_pawn_structure(board, weights, masks, Color::White) - evaluate_pawn_structure(board, weights, masks, Color::Black);

    score += evaluate_bishop_pair(board, weights, Color::White) - evaluate_bishop_pair(board, weights, Color::Black);

    score += evaluate_mobility(board, tables, masks, weights, &mut king_info, Color::White) - evaluate_mobility(board, tables, masks, weights, &mut king_info, Color::Black);

    score += evaluate_king_safety(&king_info, weights, Color::White) - evaluate_king_safety(&king_info, weights, Color::Black);

    phase = phase.min(MAX_PHASE);
    let mut final_score: i32 = (score.mg * phase + score.eg * (MAX_PHASE - phase)) / MAX_PHASE;

    if board.side_to_move == Color::Black {
        final_score = -final_score;
    }

    final_score
}