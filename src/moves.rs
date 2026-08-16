use crate::{attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::Board};

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct Move(u16);

#[repr(u8)]
pub enum MoveFlag {
    Quiet               = 0b0000,
    DoublePush          = 0b0001,
    
    KingSideCastle      = 0b0010,
    QueenSideCastle     = 0b0011,

    Capture             = 0b0100,
    EnPassant           = 0b0101,

    PromotionN          = 0b1000,
    PromotionB          = 0b1001,
    PromotionR          = 0b1010,
    PromotionQ          = 0b1011,

    PromotionCaptureN   = 0b1100,
    PromotionCaptureB   = 0b1101,
    PromotionCaptureR   = 0b1110,
    PromotionCaptureQ   = 0b1111,
}

impl Move {
    pub const NULL: Self = Self(0);

    pub const fn new(from: u8, to: u8, flag: MoveFlag) -> Self {
        Self(from as u16 | (to as u16) << 6 | (flag as u16) << 12)
    }

    pub const fn get_from(self) -> u8 { (self.0 & 0x3F) as u8 }
    pub const fn get_to(self) -> u8 { ((self.0 >> 6) & 0x3F) as u8 }

    pub const fn get_flag(self) -> MoveFlag { 
        match (self.0 >> 12) & 0x0F {
            0b0000 => MoveFlag::Quiet,
            0b0001 => MoveFlag::DoublePush,
            0b0010 => MoveFlag::KingSideCastle,
            0b0011 => MoveFlag::QueenSideCastle,
            0b0100 => MoveFlag::Capture,
            0b0101 => MoveFlag::EnPassant,
            0b1000 => MoveFlag::PromotionN,
            0b1001 => MoveFlag::PromotionB,
            0b1010 => MoveFlag::PromotionR,
            0b1011 => MoveFlag::PromotionQ,
            0b1100 => MoveFlag::PromotionCaptureN,
            0b1101 => MoveFlag::PromotionCaptureB,
            0b1110 => MoveFlag::PromotionCaptureR,
            0b1111 => MoveFlag::PromotionCaptureQ,
            _      => unreachable!(),
        }
    }
}

fn generate_pawn_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm = board.side_to_move;
    let mut pawns = board.pieces[stm as usize][PieceType::Pawn as usize];
    let empty = !board.all_occupancy();
    let enemy = board.occupancy(stm.opposite());

    let (push, start_rank, promo_rank) = match stm {
        Color::White => (8, 1u8, 7u8),
        Color::Black => (-8, 6u8, 0u8)
    };

    while let Some(from) = pawns.pop_lsb() {
        // Single Push + Promotion & Double Push
        let to: u8 = (from as i8 + push) as u8;
        if empty.is_set(to) {
            _push_pawn_move(moves, from, to, promo_rank, false);
            if from / 8 == start_rank {
                let to: u8 = (from as i8 + 2 * push) as u8;
                if empty.is_set(to) {
                    moves.push(Move::new(from, to, MoveFlag::DoublePush));
                }
            }
        }
        // Captures + Promotion Captures + En Passant
        let mut targets = tables.get_pawn_attacks(from, stm) & enemy;
        while let Some(to) = targets.pop_lsb() {
            _push_pawn_move(moves, from, to, promo_rank, true);
        }
        if let Some(to) = board.en_passant {
            if tables.get_pawn_attacks(from, stm).is_set(to) {
                moves.push(Move::new(from, to, MoveFlag::EnPassant));
            }
        }
    }
}

fn _push_pawn_move(moves: &mut Vec<Move>, from: u8, to: u8, promo_rank: u8, is_capture: bool) {
    if is_capture {
        if to / 8 == promo_rank { // Promotion Capture
            for flag in [MoveFlag::PromotionCaptureN, MoveFlag::PromotionCaptureB, MoveFlag::PromotionCaptureR, MoveFlag::PromotionCaptureQ] {
                moves.push(Move::new(from, to, flag));
            }
        } else {
            moves.push(Move::new(from, to, MoveFlag::Capture));  
        }
    } else {
        if to / 8 == promo_rank { // Promotion
            for flag in [MoveFlag::PromotionN, MoveFlag::PromotionB, MoveFlag::PromotionR, MoveFlag::PromotionQ] {
                moves.push(Move::new(from, to, flag));
            }
        } else {
            moves.push(Move::new(from, to, MoveFlag::Quiet));  
        }
    }
}

fn is_square_attacked(square: u8, by: Color, board: &Board, tables: &Tables) -> bool {
    let occ: Bitboard = board.all_occupancy();
    let pieces: &[Bitboard; 6] = &board.pieces[by as usize];

    if (tables.get_knight_attacks(square) & pieces[PieceType::Knight as usize]) != Bitboard::EMPTY { return true; }
    if (tables.get_king_attacks(square) & pieces[PieceType::King as usize]) != Bitboard::EMPTY { return true; }
    if (tables.get_pawn_attacks(square, by.opposite()) & pieces[PieceType::Pawn as usize]) != Bitboard::EMPTY { return true; }

    let diagonal_attackers: Bitboard = pieces[PieceType::Bishop as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_bishop_attacks(square, occ) & diagonal_attackers) != Bitboard::EMPTY { return true; }

    let line_attackers: Bitboard = pieces[PieceType::Rook as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_rook_attacks(square, occ) & line_attackers) != Bitboard::EMPTY { return true; }

    false
}

fn generate_knight_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut knights: Bitboard = board.pieces[stm as usize][PieceType::Knight as usize];
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = knights.pop_lsb() {
        let mut targets = tables.get_knight_attacks(from as u8) & enemy;
        while let Some(to) = targets.pop_lsb() {
            let flag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_king_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut kings: Bitboard = board.pieces[stm as usize][PieceType::King as usize];
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = kings.pop_lsb() {
        let mut targets: Bitboard = tables.get_king_attacks(from as u8) & enemy;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }

    let occupied = board.all_occupancy();
    
    // Kingside castle
    let (right_bit, king_from, empty_mask, safe_squares) = match stm {
        Color::White => (0b0001u8, 4u8, (1u64 << 5) | (1u64 << 6), [5u8, 6u8]),
        Color::Black => (0b0100u8, 60u8, (1u64 << 61) | (1u64 << 62), [61u8, 62u8]),
    };
    if board.castling_rights & right_bit == 0 { return; }
    if occupied.0 & empty_mask != 0 { return; }
    if is_square_attacked(king_from, stm.opposite(), board, tables) { return; }
    if safe_squares.iter().any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables)) { return; }
    moves.push(Move::new(king_from, king_from + 2, MoveFlag::KingSideCastle));

    // Queenside castle
    let (right_bit, king_from, empty_mask, safe_squares) = match stm {
        Color::White => (0b0010u8, 4u8, (1u64 << 3) | (1u64 << 2) | (1u64 << 1), [3u8, 2u8]),
        Color::Black => (0b1000u8, 60u8, (1u64 << 59) | (1u64 << 58) | (1u64 << 57), [59u8, 58u8]),
    };
    if board.castling_rights & right_bit == 0 { return; }
    if occupied.0 & empty_mask != 0 { return; }
    if is_square_attacked(king_from, stm.opposite(), board, tables) { return; }
    if safe_squares.iter().any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables)) { return; }
    moves.push(Move::new(king_from, king_from - 2, MoveFlag::QueenSideCastle));
}

fn generate_rook_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut rooks: Bitboard = board.pieces[stm as usize][PieceType::Rook as usize];
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = rooks.pop_lsb() {
        let mut targets: Bitboard = tables.get_rook_attacks(from, board.all_occupancy()) & enemy;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_bishop_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut bishops: Bitboard = board.pieces[stm as usize][PieceType::Bishop as usize];
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = bishops.pop_lsb() {
        let mut targets: Bitboard = tables.get_bishop_attacks(from, board.all_occupancy()) & enemy;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_queen_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut queens: Bitboard = board.pieces[stm as usize][PieceType::Queen as usize];
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = queens.pop_lsb() {
        let mut targets: Bitboard = tables.get_queen_attacks(from, board.all_occupancy()) & enemy;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}