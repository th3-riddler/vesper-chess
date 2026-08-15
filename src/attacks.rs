use crate::bitboard::{Bitboard, Color};

/* Masks the attacks for a pawn at the given square */
fn mask_pawn_attacks(square: u8, color: Color) -> Bitboard {
    if square >= 64 {
        return Bitboard::EMPTY;
    }

    let mut attacks: Bitboard = Bitboard::EMPTY;
    let pos: u64 = 1u64 << square; // Create a bitboard with a single bit set at the pawn's position

    match color {
        Color::White => {
            if square % 8 != 0 { attacks.0 |= pos << 7 }
            if square % 8 != 7 { attacks.0 |= pos << 9 }
        },
        Color::Black => {
            if square % 8 != 0 { attacks.0 |= pos >> 7 }
            if square % 8 != 7 { attacks.0 |= pos >> 9 }
        }
    }

    attacks
}

fn mask_king_attacks(square: u8) -> Bitboard {
    if square >= 64 {
        return Bitboard::EMPTY;
    }

    let mut attacks: Bitboard = Bitboard::EMPTY;
    let pos: u64 = 1u64 << square;

    if square % 8 > 0 {
        attacks.0 |= pos >> 1;
        attacks.0 |= pos >> 9;
        attacks.0 |= pos << 7;
    }
    if square % 8 < 7 {
        attacks.0 |= pos << 1;
        attacks.0 |= pos << 9;
        attacks.0 |= pos >> 7;
    }
    attacks.0 |= pos >> 8;
    attacks.0 |= pos << 8;

    attacks
}

fn mask_knight_attacks(square: u8) -> Bitboard {
    if square >= 64 {
        return Bitboard::EMPTY;
    }

    let mut attacks: Bitboard = Bitboard::EMPTY;
    let pos: u64 = 1u64 << square;

    if square % 8 > 0 && square / 8 < 6 { 
        attacks.0 |= pos << 15;
    }
    if square % 8 < 7 && square / 8 < 6 {
        attacks.0 |= pos << 17;
    }
    if square % 8 > 0 && square / 8 > 1 {
        attacks.0 |= pos >> 17;
    }
    if square % 8 < 7 && square / 8 > 1 {
        attacks.0 |= pos >> 15;
    }
    if square % 8 > 1 && square / 8 < 7 {
        attacks.0 |= pos << 6;
    }
    if square % 8 < 6 && square / 8 < 7 {
        attacks.0 |= pos << 10;
    }
    if square % 8 > 1 && square / 8 > 0 {
        attacks.0 |= pos >> 10;
    }
    if square % 8 < 6 && square / 8 > 0 {
        attacks.0 |= pos >> 6;
    }

    attacks
}