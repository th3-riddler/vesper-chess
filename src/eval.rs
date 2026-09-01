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
            piece_values_mg: [97, 426, 394, 512, 1107, 0],
            piece_values_eg: [162, 484, 469, 850, 1473, 0],
            pst_mg: [
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 73, 87, 78, 108, 93, 63, -32, -56, -10, -2, 35, 8, 31,
                    124, 72, 17, -17, 6, 24, 29, 65, 53, 35, 13, -25, -14, 11, 40, 43, 23, 4, -5,
                    -23, -24, 6, 8, 29, 18, 25, 9, -24, -19, -12, -10, 11, 41, 43, 0, 0, 0, 0, 0,
                    0, 0, 0, 0,
                ],
                [
                    -185, -126, -71, -27, 13, -69, -135, -134, -36, -7, 31, 57, 15, 125, 5, 19, 2,
                    54, 73, 88, 120, 129, 78, 28, 1, 27, 60, 101, 60, 91, 24, 44, -15, 5, 38, 37,
                    56, 45, 31, 1, -50, -9, 22, 29, 47, 29, 26, -15, -53, -45, -19, 13, 7, 12, -12,
                    -14, -117, -37, -44, -19, -27, -5, -30, -84,
                ],
                [
                    0, -39, -30, -86, -73, -45, -26, -46, 4, 15, 9, 6, 18, 10, -2, 28, 28, 51, 43,
                    58, 31, 80, 59, 50, 15, 28, 42, 58, 58, 42, 26, 9, 13, 5, 23, 58, 50, 20, 23,
                    29, 16, 34, 36, 34, 42, 47, 46, 38, 34, 45, 46, 20, 35, 52, 72, 43, 13, 47, 35,
                    19, 24, 16, 40, 28,
                ],
                [
                    64, 39, 40, 29, 23, 49, 45, 80, 67, 45, 80, 100, 74, 111, 97, 139, 36, 69, 58,
                    56, 95, 101, 162, 121, 30, 41, 42, 52, 48, 60, 72, 72, 12, 4, 22, 34, 33, 17,
                    53, 37, 9, 13, 34, 36, 40, 42, 74, 56, 18, 24, 54, 53, 61, 59, 82, 37, 44, 49,
                    68, 76, 79, 74, 84, 45,
                ],
                [
                    -9, -27, -21, -11, -29, -27, 42, -5, 27, -14, -11, -35, -73, -5, -15, 56, 33,
                    23, 5, -3, -13, 17, 39, 61, 10, 29, 21, 13, -2, 2, 11, 25, 32, 18, 22, 43, 36,
                    23, 32, 39, 30, 41, 44, 44, 45, 47, 61, 44, 35, 42, 58, 65, 65, 73, 80, 96, 31,
                    34, 54, 64, 62, 31, 66, 47,
                ],
                [
                    17, 5, 34, -5, -35, 22, 38, 105, -101, -7, -50, 89, 56, 49, 56, 15, -129, 57,
                    -2, -2, 27, 114, 59, -18, -83, -32, -45, -94, -80, -47, -79, -151, -82, -40,
                    -51, -94, -92, -51, -91, -170, -35, 14, -28, -40, -28, -30, -6, -67, 61, 32,
                    17, -28, -31, -6, 39, 32, 41, 92, 66, -68, 5, -41, 62, 54,
                ],
            ],
            pst_eg: [
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 232, 220, 215, 136, 133, 161, 218, 224, 117, 125, 73,
                    24, 10, 31, 81, 84, 70, 52, 30, 7, 2, 15, 38, 34, 38, 36, 16, 6, 7, 11, 21, 8,
                    29, 32, 18, 18, 19, 15, 11, 9, 39, 32, 26, 23, 38, 19, 12, 5, 0, 0, 0, 0, 0, 0,
                    0, 0,
                ],
                [
                    -88, -8, 17, -1, 4, -25, 3, -109, -15, 15, 16, 19, 6, -17, 8, -37, 10, 25, 40,
                    40, 11, 11, 9, -5, 29, 44, 49, 51, 55, 47, 40, 13, 28, 41, 52, 63, 57, 39, 33,
                    15, 8, 28, 27, 54, 43, 28, 18, 10, 4, 13, 29, 31, 30, 16, 8, 15, -6, -12, 12,
                    16, 21, -4, -14, -19,
                ],
                [
                    24, 42, 32, 47, 42, 23, 25, 18, 10, 16, 26, 31, 15, 21, 21, 6, 41, 33, 26, 17,
                    20, 20, 24, 32, 40, 43, 33, 36, 16, 29, 24, 33, 32, 36, 37, 33, 22, 29, 33, 11,
                    23, 43, 38, 35, 48, 32, 24, 10, 25, 12, 6, 31, 22, 14, 23, -4, 5, 17, 16, 28,
                    27, 31, 4, -15,
                ],
                [
                    64, 76, 78, 80, 70, 75, 70, 61, 76, 90, 96, 84, 83, 65, 58, 39, 72, 67, 71, 64,
                    44, 47, 29, 27, 79, 75, 76, 65, 50, 45, 46, 45, 76, 68, 70, 63, 59, 60, 41, 42,
                    64, 56, 59, 60, 48, 45, 12, 21, 61, 59, 57, 61, 51, 38, 26, 36, 64, 57, 61, 48,
                    45, 50, 39, 44,
                ],
                [
                    75, 71, 96, 74, 102, 125, 52, 65, 73, 92, 117, 126, 163, 114, 116, 112, 72, 68,
                    78, 84, 100, 105, 82, 100, 84, 75, 56, 70, 75, 87, 122, 110, 76, 86, 75, 80,
                    62, 81, 94, 103, 63, 81, 77, 70, 70, 87, 87, 72, 54, 61, 61, 70, 75, 31, 2,
                    -20, 60, 55, 56, 86, 49, 52, 20, 16,
                ],
                [
                    -141, -73, -48, -13, -12, -13, -22, -144, -21, 29, 52, 20, 40, 63, 48, -7, -2,
                    37, 60, 71, 74, 62, 63, 6, -20, 33, 65, 81, 85, 73, 60, 14, -29, 15, 49, 77,
                    65, 50, 27, 0, -49, -10, 27, 37, 36, 22, -2, -18, -66, -21, -5, 10, 11, -6,
                    -37, -62, -118, -95, -52, -25, -51, -35, -80, -125,
                ],
            ],
            doubled_pawn_mg: -9,
            doubled_pawn_eg: -37,
            isolated_pawn_mg: -32,
            isolated_pawn_eg: -10,
            passed_pawn_bonus: [0, -1, -2, 23, 58, 99, 57, 0],
            bishop_pair_mg: 45,
            bishop_pair_eg: 87,
            mobility_mg: [0, 2, 9, 7, -1, 0],
            mobility_eg: [0, 7, 10, 7, 21, 0],
            inner_ring_weight: [0, 5, 6, 4, 5, 0],
            outer_ring_weight: [0, 1, 0, 2, 3, 0],
            king_danger_table: [
                4, 5, 1, 2, 4, 7, 12, 13, 6, 16, 20, 12, 26, 28, 28, 44, 55, 43, 60, 56, 78, 94,
                89, 102, 105, 125, 134, 143, 142, 176, 173, 209, 216, 220, 221, 258, 264, 275, 279,
                313, 270, 336, 352, 323, 312, 364, 364, 375, 401, 410, 363, 451, 413, 373, 422,
                452, 465, 447, 432, 468, 440, 437, 472, 600,
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