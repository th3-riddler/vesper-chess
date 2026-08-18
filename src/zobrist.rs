use std::sync::OnceLock;

use crate::bitboard::{Color, PieceType};

pub struct ZobristKeys {
    piece_square: [[[u64; 64]; 6]; 2],
    side_to_move: u64,
    castling: [u64; 16],
    en_passant_file: [u64; 8],
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x: u64 = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;

    x
}

impl ZobristKeys {
    fn new() -> Self {
        let mut seed = 0xD1B5_4A32_D192_ED03u64;
        let mut next = || xorshift64(&mut seed);

        let mut piece_square = [[[0u64; 64]; 6]; 2];
        for c in 0..2 {
            for p in 0..6 {
                for s in 0..64 {
                    piece_square[c][p][s] = next();
                }
            }
        }

        let side_to_move = next();

        let mut castling = [0u64; 16];
        castling.iter_mut().for_each(|k| *k = next());

        let mut en_passant_file = [0u64; 8];
        en_passant_file.iter_mut().for_each(|k| *k = next());

        ZobristKeys {
            piece_square,
            side_to_move,
            castling,
            en_passant_file,
        }
    }

    pub fn piece(&self, color: Color, piece: PieceType, square: u8) -> u64 {
        self.piece_square[color as usize][piece as usize][square as usize]
    }
    pub fn side_to_move(&self) -> u64 {
        self.side_to_move
    }
    pub fn castling(&self, rights: u8) -> u64 {
        self.castling[rights as usize]
    }
    pub fn en_passant_file(&self, file: u8) -> u64 {
        self.en_passant_file[file as usize]
    }
}

static KEYS: OnceLock<ZobristKeys> = OnceLock::new();
pub fn keys() -> &'static ZobristKeys {
    KEYS.get_or_init(ZobristKeys::new)
}
