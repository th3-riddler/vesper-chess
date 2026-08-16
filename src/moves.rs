use crate::{attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::Board};

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct Move(u16);

impl Move {
    pub const NULL: Self = Self(0);

    pub const fn new(from: u8, to: u8, flag: MoveFlag) -> Self {
        Self((from as u16) | (to as u16) << 6 | (flag as u16) << 12)
    }

    pub const fn from(self) -> u8 { (self.0 & 0x3F) as u8 }
    pub const fn to(self) -> u8 { ((self.0 >> 6) & 0x3F) as u8 }
    pub fn flag(self) -> MoveFlag { MoveFlag::from_bits(((self.0 >> 12) & 0x0F) as u8) }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

impl MoveFlag {
    pub fn from_bits(bits: u8) -> MoveFlag {
        match bits {
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
    pub fn is_capture(self) -> bool {
        matches!(self, MoveFlag::Capture | MoveFlag::EnPassant |
                       MoveFlag::PromotionCaptureN | MoveFlag::PromotionCaptureB |
                       MoveFlag::PromotionCaptureR | MoveFlag::PromotionCaptureQ)
    }

    pub fn promotion_piece(self) -> Option<PieceType> {
        use MoveFlag::*;
        match self {
            PromotionN | PromotionCaptureN => Some(PieceType::Knight),
            PromotionB | PromotionCaptureB => Some(PieceType::Bishop),
            PromotionR | PromotionCaptureR => Some(PieceType::Rook),
            PromotionQ | PromotionCaptureQ => Some(PieceType::Queen),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
pub struct UndoInfo {
    piece: PieceType,
    captured: Option<PieceType>,
    castling_rights: u8,
    en_passant: Option<u8>,
    halfmove_clock: u16,
}

impl UndoInfo {
    pub fn new(piece: PieceType, captured: Option<PieceType>, castling_rights: u8, en_passant: Option<u8>, halfmove_clock: u16) -> Self {
        Self { piece, captured, castling_rights, en_passant, halfmove_clock }
    }
    pub fn piece(&self) -> PieceType { self.piece }
    pub fn captured(&self) -> Option<PieceType> { self.captured }
    pub fn castling_rights(&self) -> u8 { self.castling_rights }
    pub fn en_passant(&self) -> Option<u8> { self.en_passant }
    pub fn halfmove_clock(&self) -> u16 { self.halfmove_clock }
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
    // if (tables.get_pawn_attacks(square, by.opposite()) & pieces[PieceType::Pawn as usize]) != Bitboard::EMPTY { return true; }

    let mut pawns = pieces[PieceType::Pawn as usize];
    while let Some(from) = pawns.pop_lsb() {
        if tables.get_pawn_attacks(from, by).is_set(square) {
            return true;
        }
    }

    let diagonal_attackers: Bitboard = pieces[PieceType::Bishop as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_bishop_attacks(square, occ) & diagonal_attackers) != Bitboard::EMPTY { return true; }

    let line_attackers: Bitboard = pieces[PieceType::Rook as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_rook_attacks(square, occ) & line_attackers) != Bitboard::EMPTY { return true; }

    false
}

fn generate_knight_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut knights: Bitboard = board.pieces[stm as usize][PieceType::Knight as usize];
    let own = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = knights.pop_lsb() {
        let mut targets = tables.get_knight_attacks(from as u8) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_king_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut kings: Bitboard = board.pieces[stm as usize][PieceType::King as usize];
    let own = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());
    
    while let Some(from) = kings.pop_lsb() {
        let mut targets: Bitboard = tables.get_king_attacks(from as u8) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
    
    // Kingside castle
    // let (right_bit, king_from, empty_mask, safe_squares) = match stm {
    //     Color::White => (0b0001u8, 4u8, (1u64 << 5) | (1u64 << 6), [5u8, 6u8]),
    //     Color::Black => (0b0100u8, 60u8, (1u64 << 61) | (1u64 << 62), [61u8, 62u8]),
    // };
    // if board.castling_rights & right_bit == 0 { return; }
    // if occupied.0 & empty_mask != 0 { return; }
    // if is_square_attacked(king_from, stm.opposite(), board, tables) { return; }
    // if safe_squares.iter().any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables)) { return; }
    // moves.push(Move::new(king_from, king_from + 2, MoveFlag::KingSideCastle));
    _add_castle_kingside(board, tables, moves);
    _add_castle_queenside(board, tables, moves);

    // Queenside castle
    // let (right_bit, king_from, empty_mask, safe_squares) = match stm {
    //     Color::White => (0b0010u8, 4u8, (1u64 << 3) | (1u64 << 2) | (1u64 << 1), [3u8, 2u8]),
    //     Color::Black => (0b1000u8, 60u8, (1u64 << 59) | (1u64 << 58) | (1u64 << 57), [59u8, 58u8]),
    // };
    // if board.castling_rights & right_bit == 0 { return; }
    // if occupied.0 & empty_mask != 0 { return; }
    // if is_square_attacked(king_from, stm.opposite(), board, tables) { return; }
    // if safe_squares.iter().any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables)) { return; }
    // moves.push(Move::new(king_from, king_from - 2, MoveFlag::QueenSideCastle));
}

fn _add_castle_kingside(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let occupied: Bitboard = board.all_occupancy();

    let (right_bit, king_from, empty_mask, safe_squares) = match stm {
        Color::White => (0b0001u8, 4u8, (1u64 << 5) | (1u64 << 6), [5u8, 6u8]),
        Color::Black => (0b0100u8, 60u8, (1u64 << 61) | (1u64 << 62), [61u8, 62u8]),
    };
    if board.castling_rights & right_bit == 0 { return; }
    if occupied.0 & empty_mask != 0 { return; }
    if is_square_attacked(king_from, stm.opposite(), board, tables) { return; }
    if safe_squares.iter().any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables)) { return; }
    moves.push(Move::new(king_from, king_from + 2, MoveFlag::KingSideCastle));
}

fn _add_castle_queenside(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let occupied: Bitboard = board.all_occupancy();

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
    let own = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = rooks.pop_lsb() {
        let mut targets: Bitboard = tables.get_rook_attacks(from, board.all_occupancy()) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_bishop_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut bishops: Bitboard = board.pieces[stm as usize][PieceType::Bishop as usize];
    let own = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = bishops.pop_lsb() {
        let mut targets: Bitboard = tables.get_bishop_attacks(from, board.all_occupancy()) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_queen_moves(board: &Board, tables: &Tables, moves: &mut Vec<Move>) {
    let stm: Color = board.side_to_move;
    let mut queens: Bitboard = board.pieces[stm as usize][PieceType::Queen as usize];
    let own = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    while let Some(from) = queens.pop_lsb() {
        let mut targets: Bitboard = tables.get_queen_attacks(from, board.all_occupancy()) & !own;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            moves.push(Move::new(from, to, flag));
        }
    }
}

pub fn generate_legal_moves(board: &mut Board, tables: &Tables) -> Vec<Move> {
    let stm: Color = board.side_to_move;
    _generate_pseudo_legal_moves(board, tables)
        .into_iter()
        .filter(|&mv| {
            let undo: UndoInfo = board.make_move(mv);
            let king_sq: u8 = board.pieces[stm as usize][PieceType::King as usize].0.trailing_zeros() as u8;
            let legal: bool = !is_square_attacked(king_sq, stm.opposite(), board, tables);
            board.unmake_move(mv, undo);

            legal
        }).collect()
}

fn _generate_pseudo_legal_moves(board: &mut Board, tables: &Tables) -> Vec<Move> {
    let mut pseudo_moves: Vec<Move> = Vec::new();
    generate_knight_moves(board, tables, &mut pseudo_moves);
    generate_king_moves(board, tables, &mut pseudo_moves);
    generate_pawn_moves(board, tables, &mut pseudo_moves);
    generate_bishop_moves(board, tables, &mut pseudo_moves);
    generate_rook_moves(board, tables, &mut pseudo_moves);
    generate_queen_moves(board, tables, &mut pseudo_moves);

    pseudo_moves
}