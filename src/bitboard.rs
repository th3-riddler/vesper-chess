use std::ops::BitOr;
use std::ops::BitAnd;
use std::ops::Not;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color { White, Black }

impl Color {
    pub fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PieceType { Pawn, Knight, Bishop, Rook, Queen, King }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bitboard (pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    
    pub fn set(&mut self, square: u8) {
        self.0 |= 1 << square;
    }

    pub fn is_set(&self, square: u8) -> bool {
        (self.0 >> square) & 1 != 0
    }

    // Counts the number of set bits in the bitboard
    pub fn pop_count(&self) -> u32 {
        self.0.count_ones()
    }

    // Clear and return the least significant bit
    pub fn pop_lsb(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            let lsb_index: u8 = self.0.trailing_zeros() as u8;
            self.0 &= self.0 - 1;
            Some(lsb_index)
        }
    }
}

impl BitOr for Bitboard {
    type Output = Bitboard;

    fn bitor(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Bitboard;

    fn bitand(self, rhs: Bitboard) -> Bitboard {
        Bitboard(self.0 & rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Bitboard;

    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}

impl Iterator for Bitboard {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.pop_lsb()
    }
}