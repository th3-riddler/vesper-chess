use crate::{bitboard::{Color, PieceType}, board::Board};

pub const HIDDEN_SIZE: usize = 512;
const SCALE: i32 = 400;
const QA: i16 = 255;
const QB: i16 = 64;

#[repr(C)]
pub struct Network {
    feature_weights: [Accumulator; 768],
    feature_bias: Accumulator,
    output_weights: [i16; 2 * HIDDEN_SIZE],
    output_bias: i16,
}

pub static NNUE: Network =
    unsafe { std::mem::transmute(*include_bytes!("nnue/vesper_net_gen-3.bin")) };

#[derive(Clone, Copy)]
#[repr(C, align(64))]
pub struct Accumulator {
    vals: [i16; HIDDEN_SIZE],
}

impl Accumulator {
    pub fn new(net: &Network) -> Self {
        net.feature_bias
    }

    /// Add a feature to an accumulator.
    pub fn add_feature(&mut self, feature_idx: usize, net: &Network) {
        for (i, d) in self.vals.iter_mut().zip(&net.feature_weights[feature_idx].vals) {
            *i += *d
        }
    }

    /// Remove a feature from an accumulator.
    pub fn remove_feature(&mut self, feature_idx: usize, net: &Network) {
        for (i, d) in self.vals.iter_mut().zip(&net.feature_weights[feature_idx].vals) {
            *i -= *d
        }
    }
}

#[inline]
fn screlu(x: i16) -> i32 {
    let y = i32::from(x).clamp(0, i32::from(QA));
    y * y
}

impl Network {
    /// `us` = the accumulator for whoever is to move, `them` = the other one.
    pub fn evaluate(&self, us: &Accumulator, them: &Accumulator) -> i32 {
        let mut output: i32 = 0;
        for (&input, &weight) in us.vals.iter().zip(&self.output_weights[..HIDDEN_SIZE]) {
            output += screlu(input) * i32::from(weight);
        }
        for (&input, &weight) in them.vals.iter().zip(&self.output_weights[HIDDEN_SIZE..]) {
            output += screlu(input) * i32::from(weight);
        }

        output /= i32::from(QA);
        output += i32::from(self.output_bias);
        output *= SCALE;
        output /= i32::from(QA) * i32::from(QB);
        output
    }
}

pub fn feature_index(perspective: Color, piece_type: usize, color: Color, square: usize) -> usize {
    let friendly: bool = color == perspective;
    let piece_offset: usize = if friendly { piece_type } else { piece_type + 6 };
    let sq: usize = if perspective == Color::White { square } else { square ^ 56 };

    64 * piece_offset + sq
}

const ALL_PIECES: [PieceType; 6] = [
    PieceType::Pawn,
    PieceType::Knight,
    PieceType::Bishop,
    PieceType::Rook,
    PieceType::Queen,
    PieceType::King,
];

#[derive(Clone, Copy, Default)]
pub struct FeatureDiff {
    removed: [Option<(Color, PieceType, u8)>; 2],
    added: [Option<(Color, PieceType, u8)>; 2]
}

impl FeatureDiff {
    pub fn push_removed(&mut self, color: Color, piece: PieceType, square: u8) {
        let slot: &mut Option<(Color, PieceType, u8)> = self.removed.iter_mut().find(|s| s.is_none())
            .expect("FeatureDiff can only record 2 removals per move");

        *slot = Some((color, piece, square));
    }

    pub fn push_added(&mut self, color: Color, piece: PieceType, square: u8) {
        let slot: &mut Option<(Color, PieceType, u8)> = self.added.iter_mut().find(|s| s.is_none())
            .expect("FeatureDiff can only record 2 additions per move");

        *slot = Some((color, piece, square));
    }
}

#[derive(Clone)]
pub struct AccumulatorStack {
    stack: Vec<[Accumulator; 2]>
}

impl AccumulatorStack {
    pub fn new(board: &Board) -> Self {
        let mut pair: [Accumulator; 2] = [Accumulator::new(&NNUE), Accumulator::new(&NNUE)];

        for color in [Color::White, Color::Black] {
            for piece in ALL_PIECES {
                let mut bb = board.pieces[color as usize][piece as usize];
                while let Some(square) = bb.pop_lsb() {
                    for perspective in [Color::White, Color::Black] {
                        let idx = feature_index(perspective, piece as usize, color, square as usize);
                        pair[perspective as usize].add_feature(idx, &NNUE);
                    }
                }
            }
        }

        Self { stack: vec![pair] }
    }

    pub fn push(&mut self, diff: &FeatureDiff) {
        let mut next: [Accumulator; 2] = *self.stack.last().expect("accumulator stack is empty");

        for perspective in [Color::White, Color::Black] {
            for (color, piece, square) in diff.removed.into_iter().flatten() {
                let idx: usize = feature_index(perspective, piece as usize, color, square as usize);
                next[perspective as usize].remove_feature(idx, &NNUE);
            }

            for (color, piece, square) in diff.added.into_iter().flatten() {
                let idx: usize = feature_index(perspective, piece as usize, color, square as usize);
                next[perspective as usize].add_feature(idx, &NNUE);
            }
        }

        self.stack.push(next);
    }

    pub fn pop(&mut self) {
        self.stack.pop().expect("accumulator stack underflow: pop without matching push");
    }

    pub fn current(&self, stm: Color) -> (&Accumulator, &Accumulator) {
        let top: &[Accumulator; 2] = self.stack.last().expect("accumulator stack is empty");
        (&top[stm as usize], &top[stm.opposite() as usize])
    }
}