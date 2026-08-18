use std::time::{Duration, Instant};

use velvet::{attacks::Tables, board::{Board, square_from_index}, search::search_best_move};

fn main() {
    let tables: Tables = Tables::new();
    let mut board = Board::from_fen("r3k1nr/ppp2ppp/2n1b3/2bpp3/8/8/PPPPPqPP/RNBQKBR1 w Qkq - 0 9").unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);

    let best = search_best_move(&mut board, &tables, deadline);
    println!("Best move: from {} to {}", square_from_index(best.from()), square_from_index(best.to()));
}