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
        let rook_magics: [MagicEntry; 64] = std::array::from_fn(|sq: usize| init_rook_magic(sq as u8));
        let bishop_magics: [MagicEntry; 64] = std::array::from_fn(|sq: usize| init_bishop_magic(sq as u8));

        // _generate_magic_numbers();

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

pub const ROOK_MAGICS: [u64; 64] = [
    0x9080001184204004,
    0x0600204011028201,
    0x4080300020008018,
    0x0880080080500004,
    0x8100050042100800,
    0x0200100108040200,
    0xC080008009000A00,
    0x0080032480005100,
    0x4020800280400020,
    0x8005004000250180,
    0x40260020C0801A00,
    0x20A1000860300300,
    0x0005000801110004,
    0x0022000942000490,
    0x000A002104220008,
    0x0006000403089042,
    0x0220A08000400080,
    0x0010004008402000,
    0x8020410011012000,
    0x0010808008003004,
    0x002802800C008880,
    0x1000808004000200,
    0x8230440010020809,
    0x80080A0003064084,
    0x0020410100208000,
    0x0000200040401000,
    0x020F100080200480,
    0x1622000A0010A041,
    0x0840040180080080,
    0x0044000401082010,
    0x400A000600048928,
    0x0218004A00010C84,
    0x0440006044800980,
    0x0010802008804004,
    0x0000600480801008,
    0x0802008842002110,
    0x4002805800800401,
    0x0200920080802400,
    0x4002180924001210,
    0x2000841082000141,
    0x021030C001888002,
    0x8010004020124000,
    0x0002448012020021,
    0x82080A0020120040,
    0x0044110008010004,
    0x0B02000850860004,
    0x0400011008040002,
    0x0008040058920003,
    0x8000400020800080,
    0x4010400060008080,
    0x0011A00110048080,
    0x0102002088114200,
    0x0440180080140080,
    0x0144809400820080,
    0x0188804200010080,
    0xC0A0110484004200,
    0x004A010010C0208A,
    0x600300801A400221,
    0x00008020B0084202,
    0x00C0083410010021,
    0x002A002108100402,
    0x1411004400020801,
    0x0800C21028408104,
    0x02C4004100208C02,
];
pub const BISHOP_MAGICS: [u64; 64] = [
    0x0008010804840080,
    0x2002040408820000,
    0x44101941D3000040,
    0x0004140080000803,
    0x20D4242080052200,
    0x0088450820504028,
    0x4288480414200040,
    0x0000420490080680,
    0x0000080803240C21,
    0x0000824A280A0184,
    0x1004620820C08000,
    0x0020040400846000,
    0x0004031040104310,
    0x4004410118418020,
    0x8581040402880480,
    0x0220209044100480,
    0x0220000434100220,
    0x0010800612184900,
    0x0803810401040300,
    0x0004202802430020,
    0x4004040480A00005,
    0x1002004088040201,
    0x9500920100909000,
    0x2301022045082140,
    0x2402220010208220,
    0x0810083210020080,
    0x010C100002048154,
    0x4202016008008020,
    0x0004840004812000,
    0x2001220001004100,
    0x88040060C4011C00,
    0x0008820042804440,
    0x0010021100381040,
    0x02041A0200200406,
    0x240C020200030401,
    0x8422500820040400,
    0x8028020400001100,
    0x0C02009100220041,
    0x02C4608400821301,
    0x10040C10C0008040,
    0x0002823840102049,
    0x2882080108041400,
    0x2042802808000500,
    0x1A00084200920800,
    0x8443180103001014,
    0x0090011011000062,
    0x0860051A0A110084,
    0x0008080041C08080,
    0x1024020151182601,
    0x0002240118980000,
    0x0002024A08242002,
    0x4002100055E80000,
    0x0000043020220000,
    0x0800401026022080,
    0x1020201408888412,
    0x0010140804802100,
    0x4041003210040400,
    0xC428002108021040,
    0x4444400821841010,
    0x0010002502228800,
    0x0018E80010460602,
    0x0510001021031100,
    0x0008218410108100,
    0x082003CC08038020,
];

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
            if square % 8 != 0 { attacks.0 |= pos >> 9 }
            if square % 8 != 7 { attacks.0 |= pos >> 7 }
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
    // let (magic, relevant_bits) = _find_magic_numbers(square, mask, &ROOK_DIRS);
    let magic = ROOK_MAGICS[square as usize];
    let relevant_bits = mask.pop_count();
    let mut attacks: Vec<Option<Bitboard>> = vec![None; 1 << relevant_bits];

    for occ in enumerate_subsets(mask) {
        let real_attacks = sliding_attacks(square, occ, &ROOK_DIRS);
        let index = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
        match attacks[index] {
            None => attacks[index] = Some(real_attacks),
            Some(existing) if existing == real_attacks => {},
            Some(_) => panic!("bad rook magic for square {square}: index {index} collides")
        };
    }

    MagicEntry {
        mask,
        magic,
        shift: 64 - relevant_bits,
        attacks: attacks.into_iter().map(|a: Option<Bitboard>| a.unwrap_or_else(|| Bitboard::EMPTY)).collect(),
    }
}

/* Initializes the magic bitboard for a bishop at the given square */
fn init_bishop_magic(square: u8) -> MagicEntry {
    let mask = mask_bishop_attacks(square);
    // let (magic, relevant_bits) = _find_magic_numbers(square, mask, &BISHOP_DIRS);
    let magic: u64 = BISHOP_MAGICS[square as usize];
    let relevant_bits: u32 = mask.pop_count();
    let mut attacks: Vec<Option<Bitboard>> = vec![None; 1 << relevant_bits];

    for occ in enumerate_subsets(mask) {
        let real_attacks = sliding_attacks(square, occ, &BISHOP_DIRS);
        let index: usize = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
        match attacks[index] {
            None => attacks[index] = Some(real_attacks),
            Some(existing) if existing == real_attacks => {},
            Some(_) => panic!("bad bishop magic for square {square}: index {index} collides")
        };
    }

    MagicEntry {
        mask,
        magic,
        shift: 64 - relevant_bits,
        attacks: attacks.into_iter().map(|a: Option<Bitboard>| a.unwrap_or_else(|| Bitboard::EMPTY)).collect(),
    }
}

/* Generates sliding attacks for a piece at the given square */
fn sliding_attacks(square: u8, blockers: Bitboard, dirs: &[(i8, i8); 4]) -> Bitboard {
    let mut attacks: Bitboard = Bitboard::EMPTY;
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

/* THIS SECTION IS FOR GENERATING MAGIC NUMBERS. IT IS NOT USED IN NORMAL EXECUTION, BUT CAN BE USED TO REGENERATE THE MAGIC NUMBERS IF NEEDED. */

/* Implements the xorshift64 pseudo-random number generator */
fn _xorshift64(state: &mut u64) -> u64 {
    let mut x: u64 = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/* Finds magic numbers for the given square and mask */
fn _find_magic_numbers(square: u8, mask: Bitboard, dirs: &[(i8, i8); 4]) -> (u64, u32) {
    let relevant_bits: u32 = mask.pop_count();
    let occupancies: Vec<Bitboard> = enumerate_subsets(mask);
    let attacks: Vec<Bitboard> = occupancies
                                .iter()
                                .map(|&occ| sliding_attacks(square, occ, dirs))
                                .collect();
    
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15u64 ^ (square as u64);
    loop {
        let magic: u64 = _xorshift64(&mut seed) & _xorshift64(&mut seed) & _xorshift64(&mut seed);
        let mut table: Vec<Option<Bitboard>> = vec![None; 1 << relevant_bits];
        let mut ok: bool = true;
        for (i, &occ) in occupancies.iter().enumerate() {
            let index: usize = (occ.0.wrapping_mul(magic) >> (64 - relevant_bits)) as usize;
            match table[index] {
                None => table[index] = Some(attacks[i]),
                Some(existing) if existing == attacks[i] => {},
                Some(_) => { ok = false; break; }
            }
        }
        if ok { return (magic, relevant_bits); }
    }
}

/* Generates magic numbers for rook and bishop attacks */
fn _generate_magic_numbers() {
    println!("pub const ROOK_MAGICS: [u64; 64] = [");
    for square in 0u8..64 {
        let (magic, _bits) = _find_magic_numbers(square, mask_rook_attacks(square), &ROOK_DIRS);
        println!("  0x{:016X},", magic);
    }
    println!("];");

    println!("pub const BISHOP_MAGICS: [u64; 64] = [");
    for square in 0u8..64 {
        let (magic, _bits) = _find_magic_numbers(square, mask_bishop_attacks(square), &BISHOP_DIRS);
        println!("  0x{:016X},", magic);
    }
    println!("];");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rook_magic_are_correct() {
        let tables: Tables = Tables::new();
        for square in 0u8..64 {
            for occ in enumerate_subsets(mask_rook_attacks(square)) {
                let expected: Bitboard = sliding_attacks(square, occ, &ROOK_DIRS);
                let actual: Bitboard = tables.get_rook_attacks(square, occ);
                assert_eq!(actual, expected, "square {square}, occ {:#018X}", occ.0);
            }
        }
    }

    #[test]
    fn bishop_magic_are_correct() {
        let tables: Tables = Tables::new();
        for square in 0u8..64 {
            for occ in enumerate_subsets(mask_bishop_attacks(square)) {
                let expected: Bitboard = sliding_attacks(square, occ, &BISHOP_DIRS);
                let actual: Bitboard = tables.get_bishop_attacks(square, occ);
                assert_eq!(actual, expected, "square {square}, occ {:#018X}", occ.0);
            }
        }
    }
}