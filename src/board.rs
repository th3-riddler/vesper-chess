use std::str::SplitWhitespace;

use crate::{
    bitboard::{Bitboard, Color, PieceType}, moves::{Move, MoveFlag, UndoInfo}, nnue::{self, Accumulator, NNUE}, uci::{square_from_algebraic, square_to_algebraic}, zobrist::{ZobristKeys, keys},
};

#[derive(Clone, Copy)]
pub struct Board {
    pub pieces: [[Bitboard; 6]; 2],
    pub side_to_move: Color,
    pub castling_rights: u8,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u16,
    pub fullmove_number: u16,
    pub zobrist_key: u64,
    pub accumulators: [Accumulator; 2],
}

pub struct NullMoveUndo {
    en_passant: Option<u8>,
    zobrist_key: u64,
}

impl Board {
    pub fn occupancy(&self, color: Color) -> Bitboard {
        self.pieces[color as usize]
            .iter()
            .fold(Bitboard::EMPTY, |acc, &bb| acc | bb)
    }

    pub fn all_occupancy(&self) -> Bitboard {
        self.occupancy(Color::White) | self.occupancy(Color::Black)
    }

    pub fn from_fen(fen: &str) -> Result<Board, String> {
        let mut fields: SplitWhitespace<'_> = fen.split_whitespace();
        let placement: &str = fields.next().ok_or("Missing piece placement")?;
        let side: &str = fields.next().unwrap_or("w");
        let castling: &str = fields.next().unwrap_or("-");
        let en_passant: &str = fields.next().unwrap_or("-");
        let halfmove: u16 = fields.next().unwrap_or("0").parse().unwrap_or(0);
        let fullmove: u16 = fields.next().unwrap_or("1").parse().unwrap_or(1);

        let mut board: Board = Board {
            pieces: [[Bitboard::EMPTY; 6]; 2],
            side_to_move: Color::White,
            castling_rights: 0,
            en_passant: None,
            halfmove_clock: halfmove,
            fullmove_number: fullmove,
            zobrist_key: 0,
            accumulators: [Accumulator::new(&NNUE), Accumulator::new(&NNUE)],
        };

        let mut square: i32 = 56;
        for c in placement.chars() {
            match c {
                '/' => square -= 16,
                '1'..='8' => square += c.to_digit(10).unwrap() as i32,
                _ => {
                    let (color, piece) = piece_from_fen_char(&c)
                        .ok_or_else(|| format!("Invalid piece character: {}", c))?;
                    board.place_piece(color, piece, square as u8);
                    square += 1
                }
            }
        }

        board.side_to_move = if side == "w" {
            Color::White
        } else {
            Color::Black
        };
        for c in castling.chars() {
            board.castling_rights |= match c {
                'K' => 0b0001,
                'Q' => 0b0010,
                'k' => 0b0100,
                'q' => 0b1000,
                _ => 0,
            };
        }
        board.en_passant = square_from_algebraic(en_passant);
        board.zobrist_key = board.compute_zobrist_key(); // Computes the zobrist key given the board state

        Ok(board)
    }

    pub fn start_position() -> Result<Board, String> {
        Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
    }

    pub fn piece_on(&self, color: Color, square: u8) -> Option<PieceType> {
        [
            PieceType::Pawn,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ]
        .into_iter()
        .find(|&pt| self.pieces[color as usize][pt as usize].is_set(square))
    }

    fn place_piece(&mut self, color: Color, piece: PieceType, square: u8) {
        self.pieces[color as usize][piece as usize].0 |= 1u64 << square;
        for perspective in [Color::White, Color::Black] {
            let idx: usize = nnue::feature_index(perspective, piece as usize, color, square as usize);
            self.accumulators[perspective as usize].add_feature(idx, &NNUE);
        }
    }

    fn remove_piece(&mut self, color: Color, piece: PieceType, square: u8) {
        self.pieces[color as usize][piece as usize].0 &= !(1u64 << square);
        for perspective in [Color::White, Color::Black] {
            let idx: usize = nnue::feature_index(perspective, piece as usize, color, square as usize);
            self.accumulators[perspective as usize].remove_feature(idx, &NNUE);
        }
    }

    pub fn make_move(&mut self, mv: Move) -> UndoInfo {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());
        let stm: Color = self.side_to_move;
        let opp: Color = stm.opposite();
        let keys: &ZobristKeys = keys();

        let piece: PieceType = self.piece_on(stm, from).expect("No piece on from-square");
        let captured: Option<PieceType> = match flag {
            MoveFlag::EnPassant => Some(PieceType::Pawn),
            _ if flag.is_capture() => self.piece_on(opp, to),
            _ => None,
        };

        let undo: UndoInfo = UndoInfo::new(
            piece,
            captured,
            self.castling_rights,
            self.en_passant,
            self.halfmove_clock,
            self.zobrist_key,
            self.accumulators
        );

        // Remove the piece from the from-square
        self.remove_piece(stm, piece, from);
        self.zobrist_key ^= keys.piece(stm, piece, from);

        // Handle captures and en passant
        match flag {
            MoveFlag::EnPassant => {
                let square: u8 = if stm == Color::White { to - 8 } else { to + 8 };
                self.remove_piece(opp, PieceType::Pawn, square);
                self.zobrist_key ^= keys.piece(opp, PieceType::Pawn, square);
            }
            _ if flag.is_capture() => {
                let cap: PieceType = captured.unwrap();
                self.remove_piece(opp, cap, to);
                self.zobrist_key ^= keys.piece(opp, cap, to);
            }
            _ => {}
        }

        // Update the piece on the to-square, considering promotion if applicable
        let landing: PieceType = flag.promotion_piece().unwrap_or(piece);
        self.place_piece(stm, landing, to);
        self.zobrist_key ^= keys.piece(stm, landing, to);

        // Update castling rights
        match flag {
            MoveFlag::KingSideCastle => self._move_rook_for_castle(stm, keys, true),
            MoveFlag::QueenSideCastle => self._move_rook_for_castle(stm, keys, false),
            _ => {}
        }

        self.zobrist_key ^= keys.castling(self.castling_rights);
        self.castling_rights &= _castling_rights_mask(from) & _castling_rights_mask(to);
        self.zobrist_key ^= keys.castling(self.castling_rights);

        if let Some(old_ep) = self.en_passant {
            self.zobrist_key ^= keys.en_passant_file(old_ep % 8)
        }
        self.en_passant = (flag == MoveFlag::DoublePush).then(|| {
            if stm == Color::White {
                from + 8
            } else {
                from - 8
            }
        });
        if let Some(new_ep) = self.en_passant {
            self.zobrist_key ^= keys.en_passant_file(new_ep % 8)
        }

        self.halfmove_clock = if piece == PieceType::Pawn || flag.is_capture() {
            0
        } else {
            self.halfmove_clock + 1
        };
        if stm == Color::Black {
            self.fullmove_number += 1
        };
        self.side_to_move = opp;
        self.zobrist_key ^= keys.side_to_move();

        undo
    }

    pub fn unmake_move(&mut self, mv: Move, undo: UndoInfo) {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        // Flipping 'side_to_move' back to the previous move
        self.side_to_move = self.side_to_move.opposite();
        let stm: Color = self.side_to_move;
        let opp: Color = stm.opposite();
        if stm == Color::Black {
            self.fullmove_number -= 1;
        }

        // Remove the piece from the to-square
        let landing: PieceType = flag.promotion_piece().unwrap_or(undo.piece());
        self.remove_piece(stm, landing, to);

        // Restore the piece to the from-square
        self.place_piece(stm, undo.piece(), from);

        // Restore captured piece if any
        match flag {
            MoveFlag::EnPassant => {
                let square: u8 = if stm == Color::White { to - 8 } else { to + 8 };
                self.place_piece(opp, PieceType::Pawn, square);
            }
            _ if flag.is_capture() => {
                self.place_piece(opp, undo.captured().unwrap(), to);
            }
            _ => {}
        }

        // Restore castling rights
        match flag {
            MoveFlag::KingSideCastle => self._undo_rook_for_castle(stm, true),
            MoveFlag::QueenSideCastle => self._undo_rook_for_castle(stm, false),
            _ => {}
        }

        self.en_passant = undo.en_passant();
        self.castling_rights = undo.castling_rights();
        self.halfmove_clock = undo.halfmove_clock();
        self.accumulators = undo.accumulators();
        self.zobrist_key = undo.zobrist_key();
    }

    fn _move_rook_for_castle(&mut self, stm: Color, keys: &ZobristKeys, kingside: bool) {
        let (rook_from, rook_to) = _castle_rook_squares(stm, kingside);
        self.remove_piece(stm, PieceType::Rook, rook_from);
        self.place_piece(stm, PieceType::Rook, rook_to);
        self.zobrist_key ^= keys.piece(stm, PieceType::Rook, rook_from);
        self.zobrist_key ^= keys.piece(stm, PieceType::Rook, rook_to);
    }

    fn _undo_rook_for_castle(&mut self, stm: Color, kingside: bool) {
        let (rook_from, rook_to) = _castle_rook_squares(stm, kingside);
        self.remove_piece(stm, PieceType::Rook, rook_to);
        self.place_piece(stm, PieceType::Rook, rook_from);
    }

    pub fn make_null_move(&mut self) -> NullMoveUndo {
        let undo: NullMoveUndo = NullMoveUndo {
            en_passant: self.en_passant,
            zobrist_key: self.zobrist_key
        };
        let keys: &ZobristKeys = keys();

        if let Some(ep) = self.en_passant {
            self.zobrist_key ^= keys.en_passant_file(ep % 8);
        }
        self.en_passant = None;
        self.side_to_move = self.side_to_move.opposite();
        self.zobrist_key ^= keys.side_to_move();

        undo
    }

    pub fn unmake_null_move(&mut self, undo: NullMoveUndo) {
        self.side_to_move = self.side_to_move.opposite();
        self.en_passant = undo.en_passant;
        self.zobrist_key = undo.zobrist_key;
    }

    pub fn is_insufficient_material(&self) -> bool {
        let no_king_count = |color: Color| -> u32 {
            (0..5)
                .map(|pt| self.pieces[color as usize][pt].pop_count())
                .sum()
        };

        let total = no_king_count(Color::White) + no_king_count(Color::Black);
        if total == 0 {
            return true;
        }

        let minor_pieces = |color: Color| -> u32 {
            self.pieces[color as usize][PieceType::Knight as usize].pop_count()
                + self.pieces[color as usize][PieceType::Bishop as usize].pop_count()
        };

        total == 1 && minor_pieces(Color::White) + minor_pieces(Color::Black) == 1
    }

    pub fn compute_zobrist_key(&self) -> u64 {
        let keys: &ZobristKeys = keys();
        let mut key: u64 = 0u64;

        for color in [Color::White, Color::Black] {
            for piece in [
                PieceType::Pawn,
                PieceType::Knight,
                PieceType::Bishop,
                PieceType::Rook,
                PieceType::Queen,
                PieceType::King,
            ] {
                let mut bb: Bitboard = self.pieces[color as usize][piece as usize];
                while let Some(sq) = bb.pop_lsb() {
                    key ^= keys.piece(color, piece, sq)
                }
            }
        }
        if self.side_to_move == Color::Black {
            key ^= keys.side_to_move();
        }
        key ^= keys.castling(self.castling_rights);
        if let Some(ep) = self.en_passant {
            key ^= keys.en_passant_file(ep % 8);
        }

        key
    }

    pub fn to_fen(&self) -> String {
        let mut fen: String = String::new();

        for rank in (0..8u8).rev() {
            let mut empty: u8 = 0;
            for file in 0..8u8 {
                let square: u8 = rank * 8 + file;
                let occupant: Option<(Color, PieceType)> = [Color::White, Color::Black]
                    .into_iter()
                    .find_map(|c| self.piece_on(c, square).map(|p| (c, p)));

                match occupant {
                    Some((color, piece)) => {
                        if empty > 0 {
                            fen.push_str(&empty.to_string());
                            empty = 0;
                        }
                        fen.push(piece_to_fen_char(color, piece));
                    },
                    None => empty += 1,
                }
            }
            if empty > 0 {
                fen.push_str(&empty.to_string());
            }
            if rank > 0 {
                fen.push('/');
            }
        }
    
        fen.push(' ');
        fen.push(if self.side_to_move == Color::White { 'w' } else { 'b' });

        fen.push(' ');
        let mut castling: String = String::new();
        if self.castling_rights & 0b0001 != 0 { castling.push('K'); }
        if self.castling_rights & 0b0010 != 0 { castling.push('Q'); }
        if self.castling_rights & 0b0100 != 0 { castling.push('k'); }
        if self.castling_rights & 0b1000 != 0 { castling.push('q'); }
        fen.push_str(if castling.is_empty() { "-" } else { &castling });

        fen.push(' ');
        match self.en_passant {
            Some(sq) => fen.push_str(&square_to_algebraic(sq)),
            None => fen.push('-'),
        }

        fen.push(' ');
        fen.push_str(&self.halfmove_clock.to_string());
        
        fen.push(' ');
        fen.push_str(&self.fullmove_number.to_string());

        fen
    }
}

fn _castle_rook_squares(stm: Color, kingside: bool) -> (u8, u8) {
    match (stm, kingside) {
        (Color::White, true) => (7, 5),    // H1 -> F1
        (Color::White, false) => (0, 3),   // A1 -> D1
        (Color::Black, true) => (63, 61),  // H8 -> F8
        (Color::Black, false) => (56, 59), // A8 -> D8
    }
}

fn _castling_rights_mask(square: u8) -> u8 {
    match square {
        0 => 0b1101,
        7 => 0b1110,
        4 => 0b1100,
        56 => 0b0111,
        63 => 0b1011,
        60 => 0b0011,
        _ => 0b1111,
    }
}

/* Returns the color and piece type for a given FEN character */
fn piece_from_fen_char(c: &char) -> Option<(Color, PieceType)> {
    match c {
        'r' => Some((Color::Black, PieceType::Rook)),
        'n' => Some((Color::Black, PieceType::Knight)),
        'b' => Some((Color::Black, PieceType::Bishop)),
        'q' => Some((Color::Black, PieceType::Queen)),
        'k' => Some((Color::Black, PieceType::King)),
        'p' => Some((Color::Black, PieceType::Pawn)),
        'R' => Some((Color::White, PieceType::Rook)),
        'N' => Some((Color::White, PieceType::Knight)),
        'B' => Some((Color::White, PieceType::Bishop)),
        'Q' => Some((Color::White, PieceType::Queen)),
        'K' => Some((Color::White, PieceType::King)),
        'P' => Some((Color::White, PieceType::Pawn)),
        _ => None,
    }
}

/* Returns the FEN character for a given color and piece type */
fn piece_to_fen_char(color: Color, piece: PieceType) -> char {
    match (color, piece) {
        (Color::Black, PieceType::Pawn) => 'p',
        (Color::Black, PieceType::Knight) => 'n',
        (Color::Black, PieceType::Bishop) => 'b',
        (Color::Black, PieceType::Rook) => 'r',
        (Color::Black, PieceType::Queen) => 'q',
        (Color::Black, PieceType::King) => 'k',
        (Color::White, PieceType::Pawn) => 'P',
        (Color::White, PieceType::Knight) => 'N',
        (Color::White, PieceType::Bishop) => 'B',
        (Color::White, PieceType::Rook) => 'R',
        (Color::White, PieceType::Queen) => 'Q',
        (Color::White, PieceType::King) => 'K',
    }
}
