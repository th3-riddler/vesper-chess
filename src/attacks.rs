use crate::{bitboard::{Bitboard, Color}};

pub struct Tables {
    knight_attacks: [Bitboard; 64],
    king_attacks: [Bitboard; 64],
    pawn_attacks: [[Bitboard; 64]; 2],
    rook_magics: [MagicEntry; 64],
    bishop_magics: [MagicEntry; 64],
}

impl Tables {
    pub fn new() -> Self {
        let mut knight_attacks: [Bitboard; 64] = [Bitboard::EMPTY; 64];
        let mut king_attacks: [Bitboard; 64] = [Bitboard::EMPTY; 64];
        let mut pawn_attacks: [[Bitboard; 64]; 2] = [[Bitboard::EMPTY; 64]; 2];
        for sq in 0u8..64 {
            knight_attacks[sq as usize] = mask_knight_attacks(sq);
            king_attacks[sq as usize] = mask_king_attacks(sq);
            pawn_attacks[Color::White as usize][sq as usize] = mask_pawn_attacks(sq, Color::White);
            pawn_attacks[Color::Black as usize][sq as usize] = mask_pawn_attacks(sq, Color::Black);
        }
        let rook_magics: [MagicEntry; 64] = std::array::from_fn(|sq| init_rook_magic(sq as u8));
        let bishop_magics: [MagicEntry; 64] = std::array::from_fn(|sq| init_bishop_magic(sq as u8));

        Tables { knight_attacks, king_attacks, pawn_attacks, rook_magics, bishop_magics }
    }

    pub fn get_knight_attacks(&self, square: u8) -> Bitboard { self.knight_attacks[square as usize] }
    pub fn get_king_attacks(&self, square: u8) -> Bitboard { self.king_attacks[square as usize] }
    pub fn get_pawn_attacks(&self, square: u8, color: Color) -> Bitboard { self.pawn_attacks[color as usize][square as usize] }
    pub fn get_rook_attacks(&self, square: u8, occupied: Bitboard) -> Bitboard { sliding_lookup(occupied, &self.rook_magics[square as usize]) }
    pub fn get_bishop_attacks(&self, square: u8, occupied: Bitboard) -> Bitboard { sliding_lookup(occupied, &self.bishop_magics[square as usize]) }
    pub fn get_queen_attacks(&self, square: u8, occupied: Bitboard) -> Bitboard { self.get_rook_attacks(square, occupied) | self.get_bishop_attacks(square, occupied) }
}

struct MagicEntry {
    mask: Bitboard,
    magic: u64,
    shift: u32,
    attacks: Vec<Bitboard>,
}

const ROOK_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

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

/* Masks the attacks for a king at the given square */
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

/* Masks the attacks for a knight at the given square */
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

fn sliding_lookup(occupied: Bitboard, entry: &MagicEntry) -> Bitboard {
    let index = ((occupied & entry.mask).0.wrapping_mul(entry.magic) >> entry.shift) as usize;
    entry.attacks[index]
}

/* Generates all subsets of a given bitboard */
fn enumerate_subsets(mask: Bitboard) -> Vec<Bitboard> {
    let mut subsets: Vec<Bitboard> = Vec::new();
    let mut subset: u64 = 0;
    loop {
        subsets.push(Bitboard(subset));
        subset = subset.wrapping_sub(mask.0) & mask.0;
        if subset == 0 { break; }
    }
    subsets
}

/* Masks the attacks for a rook at the given square */
fn mask_rook_attacks(square: u8) -> Bitboard {
    let (rank, file) = (square / 8, square % 8);
    let mut mask = Bitboard::EMPTY;
    for r in 1..7 { if r != rank { mask.set(r * 8 + file); } }
    for f in 1..7 { if f != file { mask.set(rank * 8 + f); } }
    mask
}

/* Masks the attacks for a bishop at the given square */
fn mask_bishop_attacks(square: u8) -> Bitboard {
    let (rank, file) = (square / 8, square % 8);
    let mut mask = Bitboard::EMPTY;
    for r in 1..7 {
        for f in 1..7 {
            if (r as i32 - rank as i32).abs() == (f as i32 - file as i32).abs() && (r != rank || f != file) {
                mask.set(r * 8 + f);
            }
        }
    }
    mask
}

/* Initializes the magic bitboard for a rook at the given square */
fn init_rook_magic(square: u8) -> MagicEntry {
    let mask = mask_rook_attacks(square);
    let (magic, relevant_bits) = find_magic_numbers(square, mask, &ROOK_DIRS);
    let mut attacks = vec![Bitboard::EMPTY; 1 << relevant_bits];

    for occ in enumerate_subsets(mask) {
        let real_attacks = sliding_attacks(square, occ, &ROOK_DIRS);
        let index = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
        attacks[index] = real_attacks;
    }

    MagicEntry { mask, magic, shift: 64 - relevant_bits, attacks }
}

/* Initializes the magic bitboard for a bishop at the given square */
fn init_bishop_magic(square: u8) -> MagicEntry {
    let mask = mask_bishop_attacks(square);
    let (magic, relevant_bits) = find_magic_numbers(square, mask, &BISHOP_DIRS);
    let mut attacks = vec![Bitboard::EMPTY; 1 << relevant_bits];

    for occ in enumerate_subsets(mask) {
        let real_attacks = sliding_attacks(square, occ, &BISHOP_DIRS);
        let index = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
        attacks[index] = real_attacks;
    }

    MagicEntry { mask, magic, shift: 64 - relevant_bits, attacks }
}

/* Generates sliding attacks for a piece at the given square */
fn sliding_attacks(square: u8, blockers: Bitboard, dirs: &[(i8, i8); 4]) -> Bitboard {
    let mut attacks = Bitboard::EMPTY;
    let (start_rank, start_file) = ((square / 8) as i8, (square % 8) as i8);
    for &(dr, df) in dirs {
        let (mut r, mut f) = (start_rank, start_file);
        loop {
            r += dr;
            f += df;
            if !(0..8).contains(&r) || !(0..8).contains(&f) { break; }
            let sq = (r * 8 + f) as u8;
            attacks.set(sq);
            if blockers.is_set(sq) { break; }
        }
    }

    attacks
}

/* Implements the Xorshift64 pseudo-random number generator */
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/* Finds magic numbers for the given square and mask */
fn find_magic_numbers(square: u8, mask: Bitboard, dirs: &[(i8, i8); 4]) -> (u64, u32) {
    let relevant_bits = mask.pop_count();
    let occupancies = enumerate_subsets(mask);
    let attacks: Vec<Bitboard> = occupancies
                                .iter()
                                .map(|&occ| sliding_attacks(square, occ, dirs))
                                .collect();
    
    let mut seed = 0x9E37_79B9_7F4A_7C15u64 ^ (square as u64);
    loop {
        let magic = xorshift64(&mut seed) & xorshift64(&mut seed) & xorshift64(&mut seed);
        let mut table: Vec<Option<Bitboard>> = vec![None; 1 << relevant_bits];
        let mut ok = true;
        for (i, &occ) in occupancies.iter().enumerate() {
            let index = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
            match table[index] {
                None => table[index] = Some(attacks[i]),
                Some(existing) if existing == attacks[i] => {},
                Some(_) => { ok = false; break; }
            }
        }
        if ok { return (magic, relevant_bits); }
    }
}