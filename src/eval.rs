use crate::{bitboard::Color, board::Board};

const PIECE_VALUES: [i32; 6] = [100, 320, 330, 500, 900, 0]; // Pawn, Knight, Bishop, Rook, Queen, King

pub fn evaluate(board: &Board) -> i32 {
    let mut score: i32 = 0;
    for piece in 0..6 {
        let white: i32 = board.pieces[Color::White as usize][piece].pop_count() as i32;
        let black: i32 = board.pieces[Color::Black as usize][piece].pop_count() as i32;

        score += PIECE_VALUES[piece] * (white - black)
    }
    if board.side_to_move == Color::White { score } else { -score }
}