use std::time::{Duration, Instant};

use felt::{attacks::Tables, board::{Board, square_from_index}, search::search_best_move, tt::TranspositionTable};

fn main() {
    let tables: Tables = Tables::new();
    let mut tt: TranspositionTable = TranspositionTable::new(512);
    let mut board: Board = Board::from_fen("r3k1nr/ppp2ppp/2n1b3/2bpp3/8/8/PPPPPqPP/RNBQKBR1 w Qkq - 0 9").unwrap();
    let mut history: Vec<u64> = Vec::new();
    let deadline: Instant = Instant::now() + Duration::from_secs(1);

    let best = search_best_move(&mut board, &tables, &mut tt, &mut history, deadline);
    println!("Best move: from {} to {}", square_from_index(best.from()), square_from_index(best.to()));
}