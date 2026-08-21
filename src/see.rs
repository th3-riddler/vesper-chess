use crate::{attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::Board, moves::{Move, MoveFlag}};

const SEE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 20_000]; // Pawn, Knight, Bishop, Rook, Queen, King

fn attackers_to(square: u8, occ: Bitboard, board: &Board, tables: &Tables) -> Bitboard {
    let mut attackers: Bitboard = Bitboard::EMPTY;

    for color in [Color::White, Color::Black] {
        let pieces: &[Bitboard; 6] = &board.pieces[color as usize];
        attackers |= tables.get_knight_attacks(square) & pieces[PieceType::Knight as usize];
        attackers |= tables.get_king_attacks(square) & pieces[PieceType::King as usize];
        attackers |= tables.get_pawn_attacks(square, color.opposite()) & pieces[PieceType::Pawn as usize];
    }

    let (white, black) = (&board.pieces[Color::White as usize], &board.pieces[Color::Black as usize]);
    let diag: Bitboard = white[PieceType::Bishop as usize] | white[PieceType::Queen as usize] | black[PieceType::Bishop as usize] | black[PieceType::Queen as usize];
    let orth: Bitboard = white[PieceType::Rook as usize] | white[PieceType::Queen as usize] | black[PieceType::Rook as usize] | black[PieceType::Queen as usize];

    attackers |= tables.get_bishop_attacks(square, occ) & diag;
    attackers |= tables.get_rook_attacks(square, occ) & orth;

    attackers
}

fn least_valuable_attacker(attackers: Bitboard, color: Color, board: &Board) -> Option<(u8, PieceType)> {
    for piece in [PieceType::Pawn, PieceType::Bishop, PieceType::Knight, PieceType::Rook, PieceType::Queen, PieceType::King] {
        let mut candidates: Bitboard = attackers & board.pieces[color as usize][piece as usize];
        if let Some(square) = candidates.pop_lsb() { return Some((square, piece)); }
    }
    None
}

pub fn see(board: &Board, tables: &Tables, mv: Move) -> i32 {
    let to: u8 = mv.to();
    let mut side: Color = board.side_to_move;

    let initial_captured_value: i32 = match mv.flag() {
        MoveFlag::EnPassant => SEE_VALUES[PieceType::Pawn as usize],
        f if f.is_capture() => {
            let victim: PieceType = board.piece_on(side.opposite(), to).expect("capture flag but no piece on to-square");
            let promotion_gain: i32 = mv
                .flag()
                .promotion_piece()
                .map_or(0, |piece| SEE_VALUES[piece as usize] - SEE_VALUES[PieceType::Pawn as usize]);
            SEE_VALUES[victim as usize] + promotion_gain
        },
        _ => return 0,
    };

    let mut occ: Bitboard = board.all_occupancy();
    if mv.flag() == MoveFlag::EnPassant {
        let captured_square: u8 = if side == Color::White { to - 8 } else { to + 8 };
        occ.0 &= !(1u64 << captured_square);
    }

    let mut gain: [i32; 32] = [0i32; 32];
    gain[0] = initial_captured_value;
    let mut count: usize = 1usize;

    let mut attacker_square: u8 = mv.from();
    let mut attacker_value: i32 = mv
        .flag()
        .promotion_piece()
        .map_or_else(
            || SEE_VALUES[board.piece_on(side, attacker_square).unwrap() as usize],
            |piece| SEE_VALUES[piece as usize],
        );

    loop {
        occ.0 &= !(1u64 << attacker_square);
        side = side.opposite();

        let attackers: Bitboard = attackers_to(to, occ, board, tables) & occ;
        let Some((next_square, next_piece)) = least_valuable_attacker(attackers, side, board) else { break; };

        gain[count] = attacker_value;
        count += 1;

        attacker_square = next_square;
        attacker_value = SEE_VALUES[next_piece as usize];
    }
    count += 1;

    let mut result: i32 = 0;
    for i in (1..count - 1).rev() {
        result = (gain[i] - result).max(0)
    }
    
    gain[0] - result
}