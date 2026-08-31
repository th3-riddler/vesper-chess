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

#[derive(Clone, Copy)]
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
pub(crate) struct KingEvalInfo {
    // Stores the number of attack points for each color's king
    // The attack points are calculated based on the number of squares of the king's inner and outer rings attacked by a piece, weighted by the piece type
    pub king_attack_points: [i32; 2],
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

// impl Default for Weights {
//     fn default() -> Self {
//         Weights {
//             piece_values_mg: PIECE_VALUES_MG,
//             piece_values_eg: PIECE_VALUES_EG,
//             pst_mg: PESTO_MG,
//             pst_eg: PESTO_EG,
//             doubled_pawn_mg: DOUBLED_PAWN_PENALTY_MG,
//             doubled_pawn_eg: DOUBLED_PAWN_PENALTY_EG,
//             isolated_pawn_mg: ISOLATED_PAWN_PENALTY_MG,
//             isolated_pawn_eg: ISOLATED_PAWN_PENALTY_EG,
//             passed_pawn_bonus: PASSED_PAWN_BONUS,
//             bishop_pair_mg: BISHOP_PAIR_BONUS_MG,
//             bishop_pair_eg: BISHOP_PAIR_BONUS_EG,
//             mobility_mg: MOBILITY_WEIGHT_MG,
//             mobility_eg: MOBILITY_WEIGHT_EG,
//             inner_ring_weight: INNER_RING_WEIGHT,
//             outer_ring_weight: OUTER_RING_WEIGHT,
//             king_danger_table: KING_DANGER_TABLE
//         }
//     }
// }

impl Default for Weights {
    fn default() -> Self {
        Weights {
            piece_values_mg: [61, 290, 256, 351, 771, 0],
            piece_values_eg: [105, 342, 329, 617, 1156, 0],
            pst_mg: [
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 50, 55, 64, 74, 79, 63, -51, -38, -10, -5, 17, 12, 20,
                    80, 54, 18, -10, -2, 13, 16, 36, 34, 24, 10, -27, -10, 5, 17, 20, 20, 8, -2,
                    -23, -16, -1, 3, 16, 15, 21, 5, -22, -15, -6, -7, 5, 27, 33, -3, 0, 0, 0, 0, 0,
                    0, 0, 0,
                ],
                [
                    -168, -122, -27, 32, 40, -98, -169, -116, -13, 4, 9, 31, 32, 56, -31, 22, 0,
                    30, 65, 64, 88, 82, 49, 27, -7, 12, 39, 58, 29, 51, 4, 31, -19, 3, 23, 28, 34,
                    26, 21, -3, -25, -11, 8, 15, 33, 16, 16, -13, -40, -31, -13, 4, 3, -3, -16,
                    -20, -66, -27, -44, -18, -20, -2, -28, -56,
                ],
                [
                    -17, -23, -58, -68, -43, -29, -33, -45, 30, 13, 2, -25, -9, -4, 34, -13, 18,
                    26, 32, 29, 23, 42, 43, 25, 9, 20, 15, 49, 30, 32, 15, 14, 2, 16, 10, 37, 28,
                    13, 10, 25, 7, 23, 30, 17, 25, 23, 27, 38, 18, 27, 33, 16, 25, 33, 42, 26, 5,
                    31, 19, 9, 9, 15, 15, 17,
                ],
                [
                    14, 32, -12, -33, 13, 13, -5, 7, 20, 20, 32, 59, 22, 69, 29, 72, 14, 13, 20,
                    26, 28, 60, 99, 59, 2, 10, 10, 20, 12, 27, 33, 27, -15, -9, -4, -1, 9, -8, 12,
                    2, -10, -9, 5, 5, 9, 12, 38, 9, -16, -8, 12, 17, 18, 18, 43, -7, 9, 9, 22, 30,
                    30, 25, 25, 12,
                ],
                [
                    -66, -15, -16, 2, -77, -74, 28, 32, 6, -21, -1, -38, -50, -8, -4, 35, 9, -7, 0,
                    -14, -24, -9, 33, 23, 7, -3, 1, -8, -11, -7, 4, 7, 3, -2, -10, 3, 3, 4, 4, 13,
                    5, 12, 6, 6, 15, 16, 31, 18, -2, 12, 24, 31, 26, 36, 43, 47, 10, 6, 21, 26, 24,
                    4, 34, 8,
                ],
                [
                    15, -12, 10, 25, -39, 26, 30, 95, -97, -8, -111, 55, 60, 57, 62, 67, -156, 26,
                    -44, -4, -8, 115, 131, 19, -42, 9, -16, -79, -72, -10, -99, -125, -46, 12, -50,
                    -52, -60, -15, -53, -129, -57, 19, -7, -22, -18, -6, 3, -62, 57, 22, 10, -15,
                    -13, -5, 35, 37, 42, 69, 42, -45, 6, -20, 48, 42,
                ],
            ],
            pst_eg: [
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 185, 183, 175, 133, 106, 127, 185, 206, 84, 88, 48, 10,
                    11, 20, 58, 60, 41, 33, 14, -5, -6, -1, 15, 15, 25, 19, 4, -5, -5, 1, 5, 3, 16,
                    15, 5, 5, 5, 4, 4, -3, 19, 20, 10, 16, 16, 8, 1, -1, 0, 0, 0, 0, 0, 0, 0, 0,
                ],
                [
                    -45, -20, -9, -16, 1, -15, -4, -85, -23, -11, 5, 6, -5, -17, -10, -54, -1, 0,
                    8, 16, 4, -2, -14, -39, 5, 28, 34, 35, 41, 26, 25, 0, 11, 23, 36, 33, 37, 22,
                    8, 1, -14, 10, 15, 30, 23, 10, -3, -10, -11, 3, 12, 8, 11, 16, -15, -9, -38,
                    -37, -5, -1, -2, -13, -21, -10,
                ],
                [
                    13, 13, 19, 21, 21, 18, 24, -3, -15, -2, 9, 17, 7, 16, -4, -5, 19, 10, 15, 10,
                    12, 12, 6, 11, 5, 12, 16, 17, 10, 13, 18, 3, 4, 17, 21, 6, 18, 22, 13, -14, 15,
                    10, 15, 20, 22, 19, 4, -8, 6, 0, -5, 13, 11, 8, 9, -5, -5, 9, 3, 8, 8, 8, -6,
                    -20,
                ],
                [
                    17, 22, 39, 42, 27, 19, 31, 30, 22, 26, 30, 15, 26, 5, 10, -1, 18, 30, 14, 8,
                    14, -13, -10, -5, 20, 16, 29, 9, 5, -4, 2, -3, 19, 21, 21, 16, 12, 14, 6, 3,
                    12, 15, -1, 12, 5, -3, -22, -16, 3, 9, 13, 8, -3, -6, -24, -2, 5, 7, 8, 7, 0,
                    0, 1, -10,
                ],
                [
                    57, 15, 53, 35, 90, 78, 1, 6, 15, 32, 37, 80, 95, 41, 39, 32, 13, 24, 39, 62,
                    72, 33, -9, 27, 26, 21, 35, 42, 49, 45, 49, 3, 14, 44, 44, 53, 48, 38, 32, 20,
                    4, 11, 31, 30, 34, 32, -2, 8, 22, -5, -6, 1, 6, -16, -29, -75, -6, -1, -14, 13,
                    -10, 7, -48, 12,
                ],
                [
                    -114, -49, -27, -2, -34, -3, -2, -138, -15, 15, 34, 16, 19, 30, 24, -4, 11, 8,
                    46, 43, 53, 49, 24, 11, -5, 20, 40, 57, 53, 47, 50, 28, -19, 3, 38, 50, 50, 36,
                    26, 20, -13, -2, 14, 31, 29, 20, 4, -1, -48, -13, 0, 6, 11, 6, -13, -34, -76,
                    -56, -36, -20, -34, -23, -47, -81,
                ],
            ],
            doubled_pawn_mg: -3,
            doubled_pawn_eg: -23,
            isolated_pawn_mg: -16,
            isolated_pawn_eg: -8,
            passed_pawn_bonus: [0, 0, 2, 18, 38, 74, 32, 0],
            bishop_pair_mg: 25,
            bishop_pair_eg: 64,
            mobility_mg: [0, 0, 5, 4, 1, 0],
            mobility_eg: [0, 1, 4, 3, 4, 0],
            inner_ring_weight: [0, 6, 7, 4, 5, 0],
            outer_ring_weight: [0, 1, 1, 2, 3, 0],
            king_danger_table: [
                36, 38, 35, 33, 37, 35, 32, 36, 39, 37, 43, 45, 44, 43, 50, 53, 58, 64, 73, 66, 81,
                82, 91, 102, 103, 105, 115, 123, 121, 143, 134, 167, 165, 172, 185, 215, 215, 219,
                183, 243, 204, 207, 256, 230, 260, 342, 288, 326, 273, 339, 256, 405, 342, 233,
                374, 417, 459, 342, 436, 406, 394, 433, 438, 521,
            ],
        }
    }
}

impl Weights {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut i32> {
        self.piece_values_mg[..5].iter_mut()
            .chain(self.piece_values_eg[..5].iter_mut())
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

pub(crate) const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
pub(crate) const MAX_PHASE: i32 = 24;

#[inline]
pub(crate) fn mirror_square(square: u8) -> u8 { square ^ 56 }

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
            let relative_rank: u8 = if color == Color::White { rank } else { 7 - rank };

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