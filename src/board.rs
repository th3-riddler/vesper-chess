use crate::{bitboard::{Bitboard, Color, PieceType}, moves::{Move, MoveFlag, UndoInfo}};

pub struct Board {
    pub pieces: [[Bitboard; 6]; 2],
    pub side_to_move: Color,
    pub castling_rights: u8,
    pub en_passant: Option<u8>,
    pub halfmove_clock: u16,
    pub fullmove_number: u16,
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
        let mut fields: std::str::SplitWhitespace<'_> = fen.split_whitespace();
        let placement: &str = fields.next().ok_or_else(|| "Missing piece placement")?;
        let side: &str = fields.next().unwrap_or_else(|| "w");
        let castling: &str = fields.next().unwrap_or_else(|| "-");
        let en_passant: &str = fields.next().unwrap_or_else(|| "-");
        let halfmove: u16 = fields.next().unwrap_or_else(|| "0").parse().unwrap_or_else(|_| 0);
        let fullmove: u16 = fields.next().unwrap_or_else(|| "1").parse().unwrap_or_else(|_| 1);

        let mut board: Board = Board {
            pieces: [[Bitboard::EMPTY; 6]; 2],
            side_to_move: Color::White,
            castling_rights: 0,
            en_passant: None,
            halfmove_clock: halfmove,
            fullmove_number: fullmove,
        };

        let mut square: i32 = 56;
        for c in placement.chars() {
            match c {
                '/' => square -= 16,
                '1'..='8' => square += c.to_digit(10).unwrap() as i32,
                _ => {
                    let (color, piece) = piece_from_fen_char(&c).ok_or_else(|| format!("Invalid piece character: {}", c))?;
                    board.pieces[color as usize][piece as usize].set(square as u8);
                    square += 1
                }
            }
        }

        board.side_to_move = if side == "w" { Color::White } else { Color::Black };
        for c in castling.chars() {
            board.castling_rights |= match c {
                'K' => 0b0001,
                'Q' => 0b0010,
                'k' => 0b0100,
                'q' => 0b1000,
                _ => 0,
            };
        }
        board.en_passant = square_from_algebraic(&en_passant);

        Ok(board)
    }

    pub fn piece_on(&self, color: Color, square: u8) -> Option<PieceType> {
        for pt in [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen, PieceType::King] {
            if self.pieces[color as usize][pt as usize].is_set(square) {
                return Some(pt);
            }
        }
        None
    }

    pub fn make_move(&mut self, mv: Move) -> UndoInfo {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());
        let stm: Color = self.side_to_move;
        let opp: Color = stm.opposite();

        let piece: PieceType = self.piece_on(stm, from).expect("No piece on from-square");
        let captured:Option<PieceType> = match flag {
            MoveFlag::EnPassant => Some(PieceType::Pawn),
            _ if flag.is_capture() => self.piece_on(opp, to),
            _ => None,
        };

        let undo: UndoInfo = UndoInfo::new(piece, captured, self.castling_rights, self.en_passant, self.halfmove_clock);

        // Remove the piece from the from-square
        self.pieces[stm as usize][piece as usize].0 &= !(1u64 << from);

        // Handle captures and en passant
        match flag {
            MoveFlag::EnPassant => {
                let square: u8 = if stm == Color::White { to - 8 } else { to + 8 };
                self.pieces[opp as usize][PieceType::Pawn as usize].0 &= !(1u64 << square);
            },
            _ if flag.is_capture() => {
                self.pieces[opp as usize][captured.unwrap() as usize].0 &= !(1u64 << to);
            },
            _ => {}
        }

        // Update the piece on the to-square, considering promotion if applicable
        let landing: PieceType = flag.promotion_piece().unwrap_or_else(|| piece);
        self.pieces[stm as usize][landing as usize].0 |= 1u64 << to;

        // Update castling rights
        match flag {
            MoveFlag::KingSideCastle => self._move_rook_for_castle(stm, true),
            MoveFlag::QueenSideCastle => self._move_rook_for_castle(stm, false),
            _ => {},
        }

        self.en_passant = (flag == MoveFlag::DoublePush).then(|| if stm == Color::White { from + 8 } else { from - 8 });
        self.castling_rights &= _castling_rights_mask(from) & _castling_rights_mask(to);
        self.halfmove_clock = if piece == PieceType::Pawn || flag.is_capture() { 0 } else { self.halfmove_clock + 1 };
        if stm == Color::Black { self.fullmove_number += 1 };
        self.side_to_move = opp;

        undo
    }

    pub fn unmake_move(&mut self, mv: Move, undo: UndoInfo) {
        let (from, to, flag) = (mv.from(), mv.to(), mv.flag());

        // Flipping 'side_to_move' back to the previous move
        self.side_to_move = self.side_to_move.opposite();
        let stm: Color = self.side_to_move;
        let opp: Color = stm.opposite();
        if stm == Color::Black { self.fullmove_number -= 1; }
        
        // Remove the piece from the to-square
        let landing: PieceType = flag.promotion_piece().unwrap_or_else(|| undo.piece());
        self.pieces[stm as usize][landing as usize].0 &= !(1u64 << to);

        // Restore the piece to the from-square
        self.pieces[stm as usize][undo.piece() as usize].0 |= 1u64 << from;

        // Restore captured piece if any
        match flag {
            MoveFlag::EnPassant => {
                let square: u8 = if stm == Color::White { to - 8 } else { to + 8 };
                self.pieces[opp as usize][PieceType::Pawn as usize].0 |= 1u64 << square;
            },
            _ if flag.is_capture() => {
                self.pieces[opp as usize][undo.captured().unwrap() as usize].0 |= 1u64 << to;
            },
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
    }

    fn _move_rook_for_castle(&mut self, stm: Color, kingside: bool) {
        let (rook_from, rook_to) = _castle_rook_squares(stm, kingside);
        self.pieces[stm as usize][PieceType::Rook as usize].0 &= !(1u64 << rook_from);
        self.pieces[stm as usize][PieceType::Rook as usize].0 |= 1u64 << rook_to;
    }

    fn _undo_rook_for_castle(&mut self, stm: Color, kingside: bool) {
        let (rook_from, rook_to) = _castle_rook_squares(stm, kingside);
        self.pieces[stm as usize][PieceType::Rook as usize].0 &= !(1u64 << rook_to);
        self.pieces[stm as usize][PieceType::Rook as usize].0 |= 1u64 << rook_from;
    }
}


fn _castle_rook_squares(stm: Color, kingside: bool) -> (u8, u8) {
    match (stm, kingside) {
        (Color::White, true) => (7, 5),   // H1 -> F1
        (Color::White, false) => (0, 3),  // A1 -> D1
        (Color::Black, true) => (63, 61), // H8 -> F8
        (Color::Black, false) => (56, 59) // A8 -> D8
    }
}

fn _castling_rights_mask(square: u8) -> u8 {
        match square {
            0  => 0b1101,
            7  => 0b1110,
            4  => 0b1100,
            56 => 0b0111,
            63 => 0b1011,
            60 => 0b0011,
            _  => 0b1111
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

/* Converts an algebraic square notation to a bitboard index (e.g., "a1" -> 0, "h8" -> 63) */
fn square_from_algebraic(ep: &str) -> Option<u8> {
    if ep.len() != 2 {
        return None;
    }
    let file = ep.chars().nth(0).unwrap();
    let rank = ep.chars().nth(1).unwrap();

    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }

    let file_index = (file as u8) - b'a';
    let rank_index = (rank as u8) - b'1';

    Some(rank_index * 8 + file_index)
}

pub fn square_from_index(index: u8) -> String {
    let file = (index % 8) as u8 + b'a';
    let rank = (index / 8) as u8 + b'1';
    format!("{}{}", file as char, rank as char)
}

// TODO: Implement a function to convert a Board back to FEN notation