use velvet::{attacks::Tables, board::Board, moves::generate_legal_moves, perft::perft};

struct Case { name: &'static str, fen: &'static str, depth: u32, expected: u64 }

const CASES: &[Case] = &[
    Case {
        name: "startpos_depth_1",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 1,
        expected: 20,
    },
    Case {
        name: "startpos_depth_3",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 3,
        expected: 8902,
    },
    Case {
        name: "startpos_depth_4",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 4,
        expected: 197_281,
    },
    Case {
        name: "startpos_depth_5",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 5,
        expected: 4_865_609,
    },
    Case {
        name: "startpos_depth_6",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 6,
        expected: 119_060_324,
    },
    Case {
        name: "kiwipete_depth_3",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 3,
        expected: 97_862,
    },
];

#[test]
fn perft_suite() {
    let tables: Tables = Tables::new();
    for case in CASES {
        let mut board: Board = Board::from_fen(case.fen).unwrap();
        let perft_result: u64 = perft(&mut board, &tables, case.depth);
        assert_eq!(perft_result, case.expected, "case '{}' failed", case.name);
    }
}

#[test]
fn detects_diagonal_pawn_pin() {
    let board = Board::from_fen("rnbqkbnr/pppp1ppp/4p3/1B6/4P3/8/PPPP1PPP/RNBQK1NR b KQkq - 1 2").unwrap();
    let tables = Tables::new();
    let d7 = 51u8; // rank 7 (index 6) * 8 + file d (index 3)
    assert!(
        generate_legal_moves(&board, &tables).iter().all(|m| m.from() != d7),
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