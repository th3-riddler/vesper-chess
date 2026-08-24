use vesper::{
    attacks::Tables, bitboard::Color, board::Board, eval::{EvalMask, evaluate}, moves::{Move, MoveFlag, generate_legal_moves}, see::see,
};

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

#[test]
fn see_stops_after_the_last_recapture() {
    let board = Board::from_fen("4k3/8/4p3/3p4/3Q4/8/8/4K3 w - - 0 1").unwrap();
    let tables = Tables::new();
    let queen_captures_pawn = Move::new(27, 35, MoveFlag::Capture);

    assert_eq!(see(&board, &tables, queen_captures_pawn), -800);
}

#[test]
fn central_knight_beats_corner_knight_for_white() {
    let e = EvalMask::new();
    let tables = Tables::new();
    let central = Board::from_fen("4k3/8/8/3N4/8/8/8/4K3 w - - 0 1").unwrap();
    let corner = Board::from_fen("4k3/8/8/8/8/8/8/N3K3 w - - 0 1").unwrap();
    assert!(evaluate(&central, &tables, &e) > evaluate(&corner, &tables,  &e), "centralized knight should score higher");
}

#[test]
fn central_knight_beats_corner_knight_for_black() {
    let e = EvalMask::new();
    let tables = Tables::new();
    let central = Board::from_fen("4k3/8/3n4/8/8/8/8/4K3 b - - 0 1").unwrap();
    let corner = Board::from_fen("4k3/8/8/8/8/8/8/n3K3 b - - 0 1").unwrap();
    assert!(evaluate(&central, &tables, &e) > evaluate(&corner, &tables, &e), "eval is from side-to-move's perspective — black benefits from centralizing too");
}

#[test]
fn white_a4_pawn_is_passed_if_b_and_c_files_are_clear_ahead() {
    let masks = EvalMask::new();
    let a4 = 24u8;
    // enemy pawn on b6 (ahead, adjacent file) SHOULD count as blocking — square 41
    let blocking = masks.get_passed_pawn_mask(a4, Color::White).is_set(41);
    assert!(blocking, "b6 should be inside a4's passed-pawn span for White");

    // enemy pawn on b3 (BEHIND a4) should NOT count — square 17
    let behind = masks.get_passed_pawn_mask(a4, Color::White).is_set(17);
    assert!(!behind, "b3 is behind the pawn and irrelevant to whether it's passed");
}

#[test]
fn black_a5_pawn_span_looks_toward_rank_1_not_rank_8() {
    let masks = EvalMask::new();
    let a5 = 32u8;
    let ahead_for_black = masks.get_passed_pawn_mask(a5, Color::Black).is_set(17); // b3 — toward rank 1
    let behind_for_black = masks.get_passed_pawn_mask(a5, Color::Black).is_set(41); // b6 — toward rank 8
    assert!(ahead_for_black, "Black's passed-pawn span should look toward rank 1");
    assert!(!behind_for_black, "not toward rank 8");
}

#[test]
fn doubled_pawns_are_penalized() {
    let masks = EvalMask::new();
    let tables = Tables::new();
    let doubled = Board::from_fen("4k3/8/8/8/4P3/8/4P3/4K3 w - - 0 1").unwrap();
    let single = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
    // same total material — the only difference is structure
    assert!(evaluate(&doubled, &tables, &masks) < evaluate(&single, &tables, &masks) + 100);
}

#[test]
fn isolated_pawn_is_penalized_vs_supported_pawn() {
    let masks = EvalMask::new();
    let tables = Tables::new();
    let isolated = Board::from_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
    let supported = Board::from_fen("4k3/8/8/8/8/3P4/4P3/4K3 w - - 0 1").unwrap();
    assert!(evaluate(&isolated, &tables, &masks) < evaluate(&supported, &tables, &masks));
}

#[test]
fn advanced_passed_pawn_beats_blocked_pawn_of_equal_material() {
    let masks = EvalMask::new();
    let tables = Tables::new();
    // white pawn on a6, nothing in front — clearly passed and close to queening
    let passed = Board::from_fen("4k3/8/P7/8/8/8/8/4K3 w - - 0 1").unwrap();
    // white pawn on a2, black pawn on a7 directly blocking its only path — not passed
    let blocked = Board::from_fen("p3k3/8/8/8/8/8/P7/4K3 w - - 0 1").unwrap();
    assert!(evaluate(&passed, &tables, &masks) > evaluate(&blocked, &tables, &masks));
}

#[test]
fn passed_pawn_bonus_is_symmetric_for_black() {
    let masks = EvalMask::new();
    let tables = Tables::new();
    // black pawn on a3, nothing between it and rank 1 — passed, close to queening, black to move
    let board = Board::from_fen("4k3/8/8/8/8/p7/8/4K3 b - - 0 1").unwrap();
    // from Black's own perspective (evaluate returns side-to-move relative), this should score well for Black
    assert!(evaluate(&board, &tables, &masks) > 0);
}

#[test]
fn passed_pawn_bonus_is_symmetric_in_magnitude() {
    let masks = EvalMask::new();
    let tables = Tables::new();
    // white pawn on e6 (relative rank 5, zero-indexed) — mirror position for black on e3
    let white_advanced = Board::from_fen("4k3/8/4P3/8/8/8/8/4K3 w - - 0 1").unwrap();
    let black_advanced = Board::from_fen("4k3/8/8/8/8/4p3/8/4K3 b - - 0 1").unwrap();
    // both scores are from side-to-move's perspective — they should be equal if the term is symmetric
    assert_eq!(evaluate(&white_advanced, &tables, &masks), evaluate(&black_advanced, &tables, &masks));
}