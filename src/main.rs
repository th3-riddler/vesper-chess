// use velvet::attacks::Tables;
use velvet::{attacks::Tables, board::{Board, square_from_index}, moves::Move, perft::{divide, perft}};


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


fn main() {
    // let _tables: Tables = Tables::new();

    let tables: Tables = Tables::new();

    // let mut board: Board = Board::from_fen(CASES[0].fen).unwrap();
    // let perft_result = divide(&mut board, &tables, CASES[0].depth);
    // // get an array with all the moves "from" and "to" squares, and the number of nodes for each move
    // for (mv, count) in &perft_result {
    //     println!("Move: from {}, to {}, Nodes: {}", square_from_index(mv.from()), square_from_index(mv.to()), count);
    // }
    // // println!("case '{}': expected {}, got {:?}", CASES[0].name, CASES[0].expected, perft_result);


    for case in CASES {
        let mut board: Board = Board::from_fen(case.fen).unwrap();
        let perft_result: u64 = perft(&mut board, &tables, case.depth);
        // let perft_result: Vec<(Move, u64)> = divide(&mut board, &tables, case.depth);
        assert_eq!(perft_result, case.expected, "case '{}' failed", case.name);
        // println!("case '{}': expected {}, got {}", case.name, case.expected, perft_result);
        // println!("case '{}': expected {}, got {:?}", case.name, case.expected, perft_result);
    }
}
