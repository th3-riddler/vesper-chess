use moss::{attacks::Tables, board::Board, moves::generate_legal_moves};

#[test]
fn detects_diagonal_pawn_pin() {
    let board =
        Board::from_fen("rnbqkbnr/pppp1ppp/4p3/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2").unwrap();
    let tables = Tables::new();
    let d7 = 51u8; // rank 7 (index 6) * 8 + file d (index 3)
    assert!(
        generate_legal_moves(&board, &tables)
            .iter()
            .all(|m| m.from() != d7),
        "d7 pawn is pinned and should have zero legal moves"
    );
}

#[test]
fn knight_can_block_or_capture_rank_check() {
    let board = Board::from_fen("4k3/8/8/8/8/1N6/8/q3K3 w - - 0 1").unwrap();
    let tables = Tables::new();
    // king on e1, checked by queen on a1 along rank 1, knight on b3 can block (c1) or capture (a1)
    assert_eq!(generate_legal_moves(&board, &tables).len(), 5); // 3 king moves + 2 knight moves
}

#[test]
fn knight_can_block_diagonal_check() {
    let board = Board::from_fen("4k3/8/8/q7/8/8/8/1N2K3 w - - 0 1").unwrap();
    let tables = Tables::new();
    // king on e1, checked by queen on a5 along the a5-e1 diagonal, knight on b1 can block on c3 or d2
    assert_eq!(generate_legal_moves(&board, &tables).len(), 6); // 4 king moves + 2 knight moves
}

#[test]
fn zobrist_incremental_matches_from_scratch() {
    fn check(board: &mut Board, tables: &Tables, depth: u32) {
        assert_eq!(
            board.zobrist_key,
            board.compute_zobrist_key(),
            "hash drifted from true value"
        );
        if depth == 0 {
            return;
        }
        for mv in generate_legal_moves(board, tables) {
            let undo = board.make_move(mv);
            check(board, tables, depth - 1);
            board.unmake_move(mv, undo);
            assert_eq!(
                board.zobrist_key,
                undo.zobrist_key(),
                "unmake didn't restore hash"
            );
        }
    }
    let mut board =
        Board::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
            .unwrap();
    check(&mut board, &Tables::new(), 3);
}
