use crate::bitboard::Color;

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
    unsafe { std::mem::transmute(*include_bytes!("nnue/vesper_net.bin")) };

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