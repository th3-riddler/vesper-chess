use moss::{attacks::Tables, board::Board, perft::perft};

struct Case {
    name: &'static str,
    fen: &'static str,
    depth: u32,
    expected: u64,
}

const CASES: &[Case] = &[
    Case {
        name: "startpos_depth_1",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 1,
        expected: 20,
    },
    Case {
        name: "startpos_depth_2",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 2,
        expected: 400,
    },
    Case {
        name: "startpos_depth_3",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 3,
        expected: 8_902,
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
        name: "kiwipete_depth_2",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 2,
        expected: 2_039,
    },
    Case {
        name: "kiwipete_depth_3",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 3,
        expected: 97_862,
    },
    Case {
        name: "kiwipete_depth_4",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 4,
        expected: 4_085_603,
    },
    Case {
        name: "kiwipete_depth_5",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 5,
        expected: 193_690_690,
    },
];

// #[test]
// fn perft_suite() {
//     let tables: Tables = Tables::new();
//     for case in CASES {
//         let mut board: Board = Board::from_fen(case.fen).unwrap();
//         let perft_result: u64 = perft(&mut board, &tables, case.depth);
//         assert_eq!(perft_result, case.expected, "case '{}' failed", case.name);
//     }
// }