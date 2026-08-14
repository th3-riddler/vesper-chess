struct Case { name: &'static str, fen: &'static str, depth: u32, expected: u64 }

const CASES: &[Case] = &[
    Case {
        name: "startpos_depth_1",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 1,
        expected: 20,
    },
    Case {
        name: "startpos_depth_4",
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        depth: 4,
        expected: 197_281,
    },
    Case {
        name: "kiwipete_depth_3",
        fen: "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth: 3,
        expected: 97_862,
    },
];