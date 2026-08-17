use crate::{attacks::Tables, board::Board, moves::{Move, UndoInfo, generate_legal_moves}};

pub fn perft(board: &mut Board, tables: &Tables, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves: Vec<crate::moves::Move> = generate_legal_moves(board, tables);
    if depth == 1 { return moves.len() as u64; }
    let mut nodes: u64 = 0;
    for mv in moves {
        let undo: UndoInfo = board.make_move(mv);
        nodes += perft(board, tables, depth - 1);
        board.unmake_move(mv, undo);
    }

    nodes
}

pub fn divide(board: &mut Board, tables: &Tables, depth: u32) -> Vec<(Move, u64)> {
    generate_legal_moves(board, tables)
        .into_iter()
        .map(|mv| {
            let undo = board.make_move(mv);
            let count = perft(board, tables, depth - 1);
            board.unmake_move(mv, undo);
            (mv, count)
        })
        .collect()
}