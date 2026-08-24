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
    king_zone: [Bitboard; 64],
}

impl EvalMask {
    pub fn new() -> Self {
        let mut file: [Bitboard; 8] = [Bitboard::EMPTY; 8];
        let mut adjacent_file: [Bitboard; 8] = [Bitboard::EMPTY; 8];
        let mut passed_pawn: [[Bitboard; 64]; 2] = [[Bitboard::EMPTY; 64]; 2];
        let mut king_zone: [Bitboard; 64] = [Bitboard::EMPTY; 64];

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
            king_zone[square as usize] = mask_king_attacks(square);
        }

        EvalMask {
            file,
            adjacent_files: adjacent_file,
            passed_pawn,
            king_zone,
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
    pub fn get_king_zone_mask(&self, square: u8) -> Bitboard {
        self.king_zone[square as usize]
    }
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

const PIECE_VALUES_MG: [i32; 6] = [82, 337, 365, 477, 1025, 0]; // Pawn, Knight, Bishop, Rook, Queen, King
const PIECE_VALUES_EG: [i32; 6] = [94, 281, 297, 512, 936, 0];

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

const DOUBLED_PAWN_PENALTY_MG: i32 = -10;
const DOUBLED_PAWN_PENALTY_EG: i32 = -20;

const ISOLATED_PAWN_PENALTY_MG: i32 = -12;
const ISOLATED_PAWN_PENALTY_EG: i32 = -8;

const PASSED_PAWN_BONUS: [i32; 8] = [0, 5, 10, 20, 35, 50, 70, 0];

const BISHOP_PAIR_BONUS_MG: i32 = 30;
const BISHOP_PAIR_BONUS_EG: i32 = 45;

const MOBILITY_WEIGHT_MG: [i32; 6] = [0, 4, 5, 3, 2, 0]; // Pawn, Knight, Bishop, Rook, Queen, King
const MOBILITY_WEIGHT_EG: [i32; 6] = [0, 3, 4, 4, 3, 0];

const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const MAX_PHASE: i32 = 24;

#[inline]
fn mirror_square(square: u8) -> u8 { square ^ 56 }

fn evaluate_piece_position(board: &Board, phase: &mut i32, color: Color) -> Score {
    let (mut mg, mut eg) = (0, 0);

    for piece in 0..6 {
        let mut p: Bitboard = board.pieces[color as usize][piece];
        while let Some(square) = p.pop_lsb() {
            let idx: usize = if color == Color::White { mirror_square(square) as usize } else { square as usize };

            mg += PIECE_VALUES_MG[piece] + PESTO_MG[piece][idx];
            eg += PIECE_VALUES_EG[piece] + PESTO_EG[piece][idx];
            *phase += PHASE_WEIGHTS[piece];
        }
    }

    Score { mg, eg }
}

fn evaluate_pawn_structure(board: &Board, color: Color, masks: &EvalMask) -> Score {
    let (mut mg, mut eg) = (0, 0);
    let stm_pawns: Bitboard = board.pieces[color as usize][PieceType::Pawn as usize];
    let opp_pawns: Bitboard = board.pieces[color.opposite() as usize][PieceType::Pawn as usize];

    // Doubled Pawns
    for f in 0u8..8 {
        let count: u32 = (stm_pawns & masks.get_file_mask(f)).pop_count();
        if count > 1 {
            let extra: i32 = (count - 1) as i32;
            
            mg += DOUBLED_PAWN_PENALTY_MG * extra;
            eg += DOUBLED_PAWN_PENALTY_EG * extra;
        }
    }

    let mut pawns: Bitboard = stm_pawns;
    while let Some(square) = pawns.pop_lsb() {
        let file: u8 = square % 8;

        // Isolated Pawns
        if stm_pawns & masks.get_adjacent_file_mask(file) == Bitboard::EMPTY {
            mg += ISOLATED_PAWN_PENALTY_MG;
            eg += ISOLATED_PAWN_PENALTY_EG;
        }
        
        // Passed Pawns
        if masks.get_passed_pawn_mask(square, color) & opp_pawns == Bitboard::EMPTY {
            let rank: u8 = square / 8;
            let relative_rank = if color == Color::White { rank } else { 7 - rank };

            mg += PASSED_PAWN_BONUS[relative_rank as usize];
            eg += PASSED_PAWN_BONUS[relative_rank as usize];
        }
    }    

    Score { mg, eg }
}

fn evaluate_bishop_pair(board: &Board, color: Color) -> Score {
    let (mut mg, mut eg) = (0, 0);

    let bishops: Bitboard = board.pieces[color as usize][PieceType::Bishop as usize];
    let has_light: bool = (bishops & Bitboard::LIGHT_SQUARES) != Bitboard::EMPTY;
    let has_dark: bool = (bishops & Bitboard::DARK_SQUARES) != Bitboard::EMPTY;

    if has_light && has_dark {
        mg += BISHOP_PAIR_BONUS_MG;
        eg += BISHOP_PAIR_BONUS_EG;
    }

    Score { mg, eg }
}

fn evaluate_mobility(board: &Board, tables: &Tables, color: Color) -> Score {
    let (mut mg, mut eg) = (0, 0);
    let own: Bitboard = board.occupancy(color);
    let occ: Bitboard = board.all_occupancy();

    for (piece, weight_mg, weight_eg) in [
        (PieceType::Knight, MOBILITY_WEIGHT_MG[1], MOBILITY_WEIGHT_EG[1]),
        (PieceType::Bishop, MOBILITY_WEIGHT_MG[2], MOBILITY_WEIGHT_EG[2]),
        (PieceType::Rook, MOBILITY_WEIGHT_MG[3], MOBILITY_WEIGHT_EG[3]),
        (PieceType::Queen, MOBILITY_WEIGHT_MG[4], MOBILITY_WEIGHT_EG[4])
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
            
            mg += weight_mg * count;
            eg += weight_eg * count;
        }
    }

    Score { mg, eg }
}

pub fn evaluate(board: &Board, tables: &Tables, masks: &EvalMask) -> i32 {
    let mut score: Score = Score { mg: 0, eg: 0 };
    let mut phase: i32 = 0;

    score += evaluate_piece_position(board, &mut phase, Color::White) - evaluate_piece_position(board, &mut phase, Color::Black);
    score += evaluate_pawn_structure(board, Color::White, masks) - evaluate_pawn_structure(board, Color::Black, masks);

    score += evaluate_bishop_pair(board, Color::White) - evaluate_bishop_pair(board, Color::Black);

    score += evaluate_mobility(board, tables, Color::White) - evaluate_mobility(board, tables, Color::Black);

    phase = phase.min(MAX_PHASE);
    let mut final_score: i32 = (score.mg * phase + score.eg * (MAX_PHASE - phase)) / MAX_PHASE;

    if board.side_to_move == Color::Black {
        final_score = -final_score;
    }

    final_score
}