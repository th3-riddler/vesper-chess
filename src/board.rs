use crate::bitboard::{Bitboard, Color, PieceType};

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

fn from_fen(fen: &str) -> Result<Board, String> {
    let mut fields = fen.split_whitespace();
    let placement = fields.next().ok_or_else(|| "Missing piece placement")?;
    let side = fields.next().unwrap_or_else(|| "w");
    let castling = fields.next().unwrap_or_else(|| "-");
    let en_passant = fields.next().unwrap_or_else(|| "-");
    let halfmove: u16 = fields.next().unwrap_or_else(|| "0").parse().unwrap_or_else(|_| 0);
    let fullmove: u16 = fields.next().unwrap_or_else(|| "1").parse().unwrap_or_else(|_| 1);

    let mut board = Board {
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

// TODO: Implement a function to convert a Board back to FEN notation