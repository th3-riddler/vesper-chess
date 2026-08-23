use crate::{
    attacks::{BISHOP_DIRS, ROOK_DIRS, Tables, sliding_attacks},
    bitboard::{Bitboard, Color, PieceType},
    board::Board,
};

#[derive(Copy, Clone, Eq, Hash, PartialEq, Debug)]
pub struct Move(u16);

impl Move {
    pub const NULL: Self = Self(0);

    pub const fn new(from: u8, to: u8, flag: MoveFlag) -> Self {
        Self((from as u16) | (to as u16) << 6 | (flag as u16) << 12)
    }

    pub const fn from(self) -> u8 {
        (self.0 & 0x3F) as u8
    }
    pub const fn to(self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }
    pub fn flag(self) -> MoveFlag {
        MoveFlag::from_bits(((self.0 >> 12) & 0x0F) as u8)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MoveFlag {
    Quiet = 0b0000,
    DoublePush = 0b0001,

    KingSideCastle = 0b0010,
    QueenSideCastle = 0b0011,

    Capture = 0b0100,
    EnPassant = 0b0101,

    PromotionN = 0b1000,
    PromotionB = 0b1001,
    PromotionR = 0b1010,
    PromotionQ = 0b1011,

    PromotionCaptureN = 0b1100,
    PromotionCaptureB = 0b1101,
    PromotionCaptureR = 0b1110,
    PromotionCaptureQ = 0b1111,
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
            _ => unreachable!(),
        }
    }
    pub fn is_capture(self) -> bool {
        matches!(
            self,
            MoveFlag::Capture
                | MoveFlag::EnPassant
                | MoveFlag::PromotionCaptureN
                | MoveFlag::PromotionCaptureB
                | MoveFlag::PromotionCaptureR
                | MoveFlag::PromotionCaptureQ
        )
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
    zobrist_key: u64,
}

impl UndoInfo {
    pub fn new( piece: PieceType, captured: Option<PieceType>, castling_rights: u8,
        en_passant: Option<u8>, halfmove_clock: u16, zobrist_key: u64,
    ) -> Self {
        Self {
            piece,
            captured,
            castling_rights,
            en_passant,
            halfmove_clock,
            zobrist_key,
        }
    }
    pub fn piece(&self) -> PieceType {
        self.piece
    }
    pub fn captured(&self) -> Option<PieceType> {
        self.captured
    }
    pub fn castling_rights(&self) -> u8 {
        self.castling_rights
    }
    pub fn en_passant(&self) -> Option<u8> {
        self.en_passant
    }
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }
    pub fn zobrist_key(&self) -> u64 {
        self.zobrist_key
    }
}

pub fn is_in_check(board: &Board, tables: &Tables) -> bool {
    let stm: Color = board.side_to_move;
    let king_sq: u8 = board.pieces[stm as usize][PieceType::King as usize]
        .0
        .trailing_zeros() as u8;
    compute_checkers(board, tables, king_sq, stm) != Bitboard::EMPTY
}

/* Compute the bitboard of all pieces that are checking the king of the side to move. */
#[inline]
pub(crate) fn compute_checkers(board: &Board, tables: &Tables, king_sq: u8, stm: Color) -> Bitboard {
    let opp: Color = stm.opposite();
    let occ: Bitboard = board.all_occupancy();
    let enemy: &[Bitboard; 6] = &board.pieces[opp as usize];

    (tables.get_knight_attacks(king_sq) & enemy[PieceType::Knight as usize])
        | (tables.get_pawn_attacks(king_sq, stm) & enemy[PieceType::Pawn as usize])
        | (tables.get_bishop_attacks(king_sq, occ)
            & (enemy[PieceType::Bishop as usize] | enemy[PieceType::Queen as usize]))
        | (tables.get_rook_attacks(king_sq, occ)
            & (enemy[PieceType::Rook as usize] | enemy[PieceType::Queen as usize]))
}

fn ray_between_inclusive(from: u8, to: u8, dirs: &[(i8, i8); 4]) -> Bitboard {
    let (from_rank, from_file) = ((from / 8) as i8, (from % 8) as i8);
    for &(dr, df) in dirs {
        let mut results = Bitboard::EMPTY;
        let (mut r, mut f) = (from_rank, from_file);
        loop {
            r += dr;
            f += df;
            if !(0..8).contains(&r) || !(0..8).contains(&f) {
                break;
            }
            let sq = (r * 8 + f) as u8;
            results.set(sq);
            if sq == to {
                return results;
            }
        }
    }
    Bitboard::EMPTY
}

/* Compute the bitboard of all pieces that are pinned to the king of the side to move. */
fn compute_pinned(
    board: &Board,
    tables: &Tables,
    king_sq: u8,
    stm: Color,
) -> (Bitboard, [Bitboard; 64]) {
    let opp: Color = stm.opposite();
    let own: Bitboard = board.occupancy(stm);
    let occ_without_own: Bitboard = board.all_occupancy() & !own;

    let mut pinned: Bitboard = Bitboard::EMPTY;
    let mut pin_rays: [Bitboard; 64] = [Bitboard::EMPTY; 64]; // Legal squares for the pinned piece on that specific square

    for (dirs, xray, pinner_pieces) in [
        (
            &ROOK_DIRS,
            tables.get_rook_attacks(king_sq, occ_without_own),
            board.pieces[opp as usize][PieceType::Rook as usize]
                | board.pieces[opp as usize][PieceType::Queen as usize],
        ),
        (
            &BISHOP_DIRS,
            tables.get_bishop_attacks(king_sq, occ_without_own),
            board.pieces[opp as usize][PieceType::Bishop as usize]
                | board.pieces[opp as usize][PieceType::Queen as usize],
        ),
    ] {
        let mut potential_pinners: Bitboard = xray & pinner_pieces;

        while let Some(pinner_sq) = potential_pinners.pop_lsb() {
            let between: Bitboard = ray_between_inclusive(king_sq, pinner_sq, dirs);
            let blockers: Bitboard = between & own;
            if blockers.pop_count() == 1 {
                let sq: u8 = blockers.0.trailing_zeros() as u8;
                pinned.set(sq);
                pin_rays[sq as usize] = between;
            }
        }
    }
    (pinned, pin_rays)
}

fn block_squares(king_sq: u8, checker_sq: u8, checker_piece: PieceType) -> Bitboard {
    match checker_piece {
        PieceType::Rook => ray_between_inclusive(king_sq, checker_sq, &ROOK_DIRS),
        PieceType::Bishop => ray_between_inclusive(king_sq, checker_sq, &BISHOP_DIRS),
        PieceType::Queen => {
            let orth: Bitboard = ray_between_inclusive(king_sq, checker_sq, &ROOK_DIRS);
            if orth.is_set(checker_sq) {
                orth
            } else {
                ray_between_inclusive(king_sq, checker_sq, &BISHOP_DIRS)
            }
        }
        _ => Bitboard::EMPTY,
    }
}

fn generate_pawn_moves(
    board: &Board,
    tables: &Tables,
    stm: Color,
    pinned: Bitboard,
    pin_rays: &[Bitboard; 64],
    target_mask: Bitboard,
    checkers: Bitboard,
    king_sq: u8,
    moves: &mut Vec<Move>,
) {
    let opp: Color = stm.opposite();
    let empty: Bitboard = !board.all_occupancy();
    let enemy: Bitboard = board.occupancy(opp);

    let mut pawns: Bitboard = board.pieces[stm as usize][PieceType::Pawn as usize];

    let (push, start_rank, promo_rank): (i8, u8, u8) = match stm {
        Color::White => (8, 1, 7),
        Color::Black => (-8, 6, 0),
    };

    while let Some(from) = pawns.pop_lsb() {
        // Single Push + Promotion & Double Push
        let allowed: Bitboard = if pinned.is_set(from) {
            pin_rays[from as usize]
        } else {
            Bitboard::ALL
        };

        let to: u8 = (from as i8 + push) as u8;
        if empty.is_set(to) {
            if target_mask.is_set(to) && allowed.is_set(to) {
                _push_pawn_move(moves, from, to, promo_rank, false);
            }
            if from / 8 == start_rank {
                let to: u8 = (from as i8 + 2 * push) as u8;
                if empty.is_set(to) && target_mask.is_set(to) && allowed.is_set(to) {
                    moves.push(Move::new(from, to, MoveFlag::DoublePush));
                }
            }
        }

        // Captures + Promotion Captures + En Passant
        let mut targets: Bitboard =
            tables.get_pawn_attacks(from, stm) & enemy & target_mask & allowed;
        while let Some(to) = targets.pop_lsb() {
            _push_pawn_move(moves, from, to, promo_rank, true);
        }
        if let Some(ep) = board.en_passant
            && tables.get_pawn_attacks(from, stm).is_set(ep)
        {
            let captured_sq: u8 = if stm == Color::White { ep - 8 } else { ep + 8 };
            let resolves_check: bool = checkers.is_set(captured_sq) || target_mask.is_set(ep);
            let stays_on_pin: bool = !pinned.is_set(from) || pin_rays[from as usize].is_set(ep);

            if resolves_check
                && stays_on_pin
                && !_en_passant_reveals_check(board, king_sq, stm, from, captured_sq)
            {
                moves.push(Move::new(from, ep, MoveFlag::EnPassant));
            }
        }
    }
}

fn _en_passant_reveals_check(
    board: &Board,
    king_sq: u8,
    stm: Color,
    from: u8,
    captured_sq: u8,
) -> bool {
    let opp: Color = stm.opposite();
    let mut occ: Bitboard = board.all_occupancy();
    occ.0 &= !(1u64 << from);
    occ.0 &= !(1u64 << captured_sq);

    let attackers: Bitboard = sliding_attacks(king_sq, occ, &ROOK_DIRS)
        & (board.pieces[opp as usize][PieceType::Rook as usize]
            | board.pieces[opp as usize][PieceType::Queen as usize]);

    attackers != Bitboard::EMPTY
}

fn _push_pawn_move(moves: &mut Vec<Move>, from: u8, to: u8, promo_rank: u8, is_capture: bool) {
    if is_capture {
        if to / 8 == promo_rank {
            // Promotion Capture
            for flag in [
                MoveFlag::PromotionCaptureN,
                MoveFlag::PromotionCaptureB,
                MoveFlag::PromotionCaptureR,
                MoveFlag::PromotionCaptureQ,
            ] {
                moves.push(Move::new(from, to, flag));
            }
        } else {
            moves.push(Move::new(from, to, MoveFlag::Capture));
        }
    } else {
        if to / 8 == promo_rank {
            // Promotion
            for flag in [
                MoveFlag::PromotionN,
                MoveFlag::PromotionB,
                MoveFlag::PromotionR,
                MoveFlag::PromotionQ,
            ] {
                moves.push(Move::new(from, to, flag));
            }
        } else {
            moves.push(Move::new(from, to, MoveFlag::Quiet));
        }
    }
}

fn is_square_attacked(
    square: u8,
    by: Color,
    board: &Board,
    tables: &Tables,
    occ: Bitboard,
) -> bool {
    let pieces: &[Bitboard; 6] = &board.pieces[by as usize];

    if (tables.get_knight_attacks(square) & pieces[PieceType::Knight as usize]) != Bitboard::EMPTY {
        return true;
    }
    if (tables.get_king_attacks(square) & pieces[PieceType::King as usize]) != Bitboard::EMPTY {
        return true;
    }
    if (tables.get_pawn_attacks(square, by.opposite()) & pieces[PieceType::Pawn as usize])
        != Bitboard::EMPTY
    {
        return true;
    }

    let diagonal_attackers: Bitboard =
        pieces[PieceType::Bishop as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_bishop_attacks(square, occ) & diagonal_attackers) != Bitboard::EMPTY {
        return true;
    }

    let line_attackers: Bitboard =
        pieces[PieceType::Rook as usize] | pieces[PieceType::Queen as usize];
    if (tables.get_rook_attacks(square, occ) & line_attackers) != Bitboard::EMPTY {
        return true;
    }

    false
}

fn generate_knight_moves(
    board: &Board,
    tables: &Tables,
    stm: Color,
    pinned: Bitboard,
    target_mask: Bitboard,
    moves: &mut Vec<Move>,
) {
    let own: Bitboard = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    let mut knights: Bitboard = board.pieces[stm as usize][PieceType::Knight as usize] & !pinned;

    while let Some(from) = knights.pop_lsb() {
        let mut targets: Bitboard = tables.get_knight_attacks(from) & !own & target_mask;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) {
                MoveFlag::Capture
            } else {
                MoveFlag::Quiet
            };
            moves.push(Move::new(from, to, flag));
        }
    }
}

fn generate_king_moves(
    board: &Board,
    tables: &Tables,
    king_sq: u8,
    stm: Color,
    checkers: Bitboard,
    moves: &mut Vec<Move>,
) {
    let opp: Color = stm.opposite();

    // let mut kings: Bitboard = board.pieces[stm as usize][PieceType::King as usize];
    let own: Bitboard = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(opp);
    let occ_without_kings: Bitboard = board.all_occupancy() & !Bitboard(1u64 << king_sq);

    let mut targets: Bitboard = tables.get_king_attacks(king_sq) & !own;
    while let Some(to) = targets.pop_lsb() {
        if !is_square_attacked(to, opp, board, tables, occ_without_kings) {
            let flag: MoveFlag = if enemy.is_set(to) {
                MoveFlag::Capture
            } else {
                MoveFlag::Quiet
            };
            moves.push(Move::new(king_sq, to, flag));
        }
    }

    if checkers == Bitboard::EMPTY {
        generate_castling_moves(board, tables, stm, moves);
    }
}

fn generate_castling_moves(board: &Board, tables: &Tables, stm: Color, moves: &mut Vec<Move>) {
    _add_castle_kingside(board, tables, stm, moves);
    _add_castle_queenside(board, tables, stm, moves);
}

fn _add_castle_kingside(board: &Board, tables: &Tables, stm: Color, moves: &mut Vec<Move>) {
    let occ: Bitboard = board.all_occupancy();

    let (right_bit, king_from, empty_mask, safe_squares) = match stm {
        Color::White => (0b0001u8, 4u8, (1u64 << 5) | (1u64 << 6), [5u8, 6u8]),
        Color::Black => (0b0100u8, 60u8, (1u64 << 61) | (1u64 << 62), [61u8, 62u8]),
    };
    if board.castling_rights & right_bit == 0 {
        return;
    }
    if occ.0 & empty_mask != 0 {
        return;
    }
    if is_square_attacked(king_from, stm.opposite(), board, tables, occ) {
        return;
    }
    if safe_squares
        .iter()
        .any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables, occ))
    {
        return;
    }
    moves.push(Move::new(
        king_from,
        king_from + 2,
        MoveFlag::KingSideCastle,
    ));
}

fn _add_castle_queenside(board: &Board, tables: &Tables, stm: Color, moves: &mut Vec<Move>) {
    let occ: Bitboard = board.all_occupancy();

    let (right_bit, king_from, empty_mask, safe_squares) = match stm {
        Color::White => (
            0b0010u8,
            4u8,
            (1u64 << 3) | (1u64 << 2) | (1u64 << 1),
            [3u8, 2u8],
        ),
        Color::Black => (
            0b1000u8,
            60u8,
            (1u64 << 59) | (1u64 << 58) | (1u64 << 57),
            [59u8, 58u8],
        ),
    };
    if board.castling_rights & right_bit == 0 {
        return;
    }
    if occ.0 & empty_mask != 0 {
        return;
    }
    if is_square_attacked(king_from, stm.opposite(), board, tables, occ) {
        return;
    }
    if safe_squares
        .iter()
        .any(|&sq| is_square_attacked(sq, stm.opposite(), board, tables, occ))
    {
        return;
    }
    moves.push(Move::new(
        king_from,
        king_from - 2,
        MoveFlag::QueenSideCastle,
    ));
}

/* Computes sliding moves for the given piece type */
fn generate_sliding_moves(
    board: &Board,
    tables: &Tables,
    stm: Color,
    piece: PieceType,
    pinned: Bitboard,
    pin_rays: &[Bitboard; 64],
    target_mask: Bitboard,
    moves: &mut Vec<Move>,
) {
    let own: Bitboard = board.occupancy(stm);
    let enemy: Bitboard = board.occupancy(stm.opposite());

    let occ: Bitboard = board.all_occupancy();
    let mut pieces: Bitboard = board.pieces[stm as usize][piece as usize];

    while let Some(from) = pieces.pop_lsb() {
        let attacks: Bitboard = match piece {
            PieceType::Bishop => tables.get_bishop_attacks(from, occ),
            PieceType::Rook => tables.get_rook_attacks(from, occ),
            PieceType::Queen => tables.get_queen_attacks(from, occ),
            _ => unreachable!(),
        };

        let allowed: Bitboard = if pinned.is_set(from) {
            pin_rays[from as usize]
        } else {
            Bitboard::ALL
        };
        let mut targets: Bitboard = attacks & !own & target_mask & allowed;
        while let Some(to) = targets.pop_lsb() {
            let flag: MoveFlag = if enemy.is_set(to) {
                MoveFlag::Capture
            } else {
                MoveFlag::Quiet
            };
            moves.push(Move::new(from, to, flag));
        }
    }
}

pub fn generate_legal_moves(board: &Board, tables: &Tables) -> Vec<Move> {
    let stm: Color = board.side_to_move;
    let king_sq: u8 = board.pieces[stm as usize][PieceType::King as usize]
        .0
        .trailing_zeros() as u8;
    let checkers: Bitboard = compute_checkers(board, tables, king_sq, stm);

    let mut moves: Vec<Move> = Vec::new();
    generate_king_moves(board, tables, king_sq, stm, checkers, &mut moves);

    // It's a double check, generates king moves only
    if checkers.pop_count() >= 2 {
        return moves;
    }

    let target_mask: Bitboard = if checkers.pop_count() == 1 {
        let checker_sq: u8 = checkers.0.trailing_zeros() as u8;
        let checker_piece: PieceType = board.piece_on(stm.opposite(), checker_sq).unwrap();

        checkers | block_squares(king_sq, checker_sq, checker_piece)
    } else {
        Bitboard::ALL
    };

    let (pinned, pin_rays) = compute_pinned(board, tables, king_sq, stm);

    generate_pawn_moves(
        board,
        tables,
        stm,
        pinned,
        &pin_rays,
        target_mask,
        checkers,
        king_sq,
        &mut moves,
    );
    generate_knight_moves(board, tables, stm, pinned, target_mask, &mut moves);
    for piece in [PieceType::Bishop, PieceType::Rook, PieceType::Queen] {
        generate_sliding_moves(
            board,
            tables,
            stm,
            piece,
            pinned,
            &pin_rays,
            target_mask,
            &mut moves,
        );
    }

    moves
}
