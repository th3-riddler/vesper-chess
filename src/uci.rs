use std::{
    io::{self, BufRead}, str::SplitWhitespace, sync::{
        Arc, Mutex, MutexGuard, atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    }, thread, time::{Duration, Instant},
};

use crate::{
    attacks::Tables, bitboard::{
        Bitboard, Color, PieceType
    }, board::Board, eval::{EvalMask, EvalMode, Weights}, moves::{
        Move,
        MoveFlag,
        generate_legal_moves
    }, search::{
        LMRTable,
        MATE_THRESHOLD,
        MATE_VALUE,
        SearchControl,
        SearchInfo,
        search_best_move
    }, tt::TranspositionTable,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn square_to_algebraic(square: u8) -> String {
    format!(
        "{}{}",
        (b'a' + square % 8) as char,
        (b'1' + square / 8) as char
    )
}

pub(crate) fn algebraic_to_square(s: &str) -> Option<u8> {
    let b: &[u8] = s.as_bytes();
    if b.len() != 2 {
        return None;
    }
    let file: u8 = b[0].checked_sub(b'a')?;
    let rank: u8 = b[1].checked_sub(b'1')?;

    (file < 8 && rank < 8).then(|| rank * 8 + file)
}

/* Converts an algebraic square notation to a bitboard index (e.g., "a1" -> 0, "h8" -> 63) */
pub(crate) fn square_from_algebraic(ep: &str) -> Option<u8> {
    if ep.len() != 2 {
        return None;
    }
    let file = ep.chars().nth(0).unwrap();
    let rank = ep.chars().nth(1).unwrap();

    if !('a'..='h').contains(&file) || !('1'..='8').contains(&rank) {
        return None;
    }

    let file_index = (file as u8) - b'a';
    let rank_index = (rank as u8) - b'1';

    Some(rank_index * 8 + file_index)
}

pub fn square_from_index(index: u8) -> String {
    let file: u8 = (index % 8) + b'a';
    let rank: u8 = (index / 8) + b'1';
    format!("{}{}", file as char, rank as char)
}

pub fn move_to_uci_string(mv: Move) -> String {
    let promo: &str = match mv.flag() {
        MoveFlag::PromotionQ | MoveFlag::PromotionCaptureQ => "q",
        MoveFlag::PromotionR | MoveFlag::PromotionCaptureR => "r",
        MoveFlag::PromotionB | MoveFlag::PromotionCaptureB => "b",
        MoveFlag::PromotionN | MoveFlag::PromotionCaptureN => "n",
        _ => "",
    };
    format!(
        "{}{}{}",
        square_to_algebraic(mv.from()),
        square_to_algebraic(mv.to()),
        promo
    )
}

pub fn parse_uci_move(s: &str, board: &Board, tables: &Tables) -> Option<Move> {
    if s.len() < 4 {
        return None;
    }
    let from: u8 = algebraic_to_square(&s[0..2])?;
    let to: u8 = algebraic_to_square(&s[2..4])?;
    let promo_piece: Option<PieceType> = match s.as_bytes().get(4) {
        Some(b'q') => Some(PieceType::Queen),
        Some(b'r') => Some(PieceType::Rook),
        Some(b'b') => Some(PieceType::Bishop),
        Some(b'n') => Some(PieceType::Knight),
        None => None,
        _ => return None,
    };

    generate_legal_moves(board, tables)
        .into_iter()
        .find(|mv: &Move| {
            mv.from() == from && mv.to() == to && mv.flag().promotion_piece() == promo_piece
        })
}

fn handle_position_command(
    tokens: &mut SplitWhitespace,
    tables: &Tables,
    history: &mut Vec<u64>,
) -> Board {
    let mut board = match tokens.next() {
        Some("fen") => {
            let fen: Vec<&str> = tokens.by_ref().take_while(|&t| t != "moves").collect();
            Board::from_fen(&fen.join(" ")).unwrap()
        }
        _ => Board::start_position().unwrap(),
    };

    history.clear();
    history.push(board.zobrist_key);

    for tok in tokens {
        if tok == "moves" {
            continue;
        }
        if let Some(mv) = parse_uci_move(tok, &board, tables) {
            board.make_move(mv);
            history.push(board.zobrist_key);
        }
    }

    board
}

pub struct GoParams {
    pub depth: Option<u32>,
    pub movetime: Option<u64>,
    pub wtime: Option<u64>,
    pub btime: Option<u64>,
    pub winc: Option<u64>,
    pub binc: Option<u64>,
}

fn parse_go_command(mut tokens: SplitWhitespace) -> GoParams {
    let mut p: GoParams = GoParams {
        depth: None,
        movetime: None,
        wtime: None,
        btime: None,
        winc: None,
        binc: None,
    };

    while let Some(tok) = tokens.next() {
        match tok {
            "depth" => {
                p.depth = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            "movetime" => {
                p.movetime = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            "wtime" => {
                p.wtime = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            "btime" => {
                p.btime = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            "winc" => {
                p.winc = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            "binc" => {
                p.binc = tokens.next().and_then(|v: &str| v.parse().ok());
            }
            _ => {}
        }
    }

    p
}

fn compute_soft_hard_ms(time: u64, inc: u64) -> (u64, u64) {
    let time_safety_cap: u64 = time.saturating_sub(20).max(10);
    let soft_ms: u64 = (time / 30 + inc / 2).max(20).min(time_safety_cap);
    let hard_ms: u64 = (soft_ms * 3).max(soft_ms + 20).min(time_safety_cap);
    
    (soft_ms, hard_ms)
}

pub fn compute_deadline(p: &GoParams, stm: Color) -> (Instant, Instant, Option<u32>) {
    let now: Instant = Instant::now();
    if let Some(d) = p.depth {
        let far_future: Instant = now + Duration::from_secs(3600);
        return (far_future, far_future, Some(d));
    }
    if let Some(mt) = p.movetime {
        let deadline: Instant = now + Duration::from_millis(mt);
        return (deadline, deadline, None);
    }

    let (time, inc) = match stm {
        Color::White => (p.wtime.unwrap_or(10_000), p.winc.unwrap_or(0)),
        Color::Black => (p.btime.unwrap_or(10_000), p.binc.unwrap_or(0)),
    };

    let (soft_ms, hard_ms) = compute_soft_hard_ms(time, inc);
    let now: Instant = Instant::now();

    (now + Duration::from_millis(soft_ms), now + Duration::from_millis(hard_ms), None)
}

fn format_score(score: i32) -> String {
    if score >= MATE_THRESHOLD {
        let plies: i32 = MATE_VALUE - score;
        format!("mate {}", (plies + 1) / 2)
    } else if score <= -MATE_THRESHOLD {
        let plies: i32 = MATE_VALUE + score;
        format!("mate -{}", (plies + 1) / 2)
    } else {
        format!("cp {score}")
    }
}

pub fn print_bitboard(bb: Bitboard) {
    for rank in (0..8u8).rev() {
        for file in 0..8u8 {
            let sq = rank * 8 + file;
            let ch = if (bb.0 >> sq) & 1 == 1 { '1' } else { '.' };
            print!("{} ", ch);
        }
        println!();
    }
    println!();
}

pub fn uci_loop() {
    let tables: Arc<Tables> = Arc::new(Tables::new());
    let mut tt: Arc<TranspositionTable> = Arc::new(TranspositionTable::new(64)); // 64 MB default
    let board: Arc<Mutex<Board>> = Arc::new(Mutex::new(Board::start_position().unwrap()));
    let weights: Arc<Weights> = Arc::new(Weights::default());
    let history: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![board.lock().unwrap().zobrist_key]));
    let lmr_table: Arc<LMRTable> = Arc::new(LMRTable::new());
    let eval_mask: Arc<EvalMask> = Arc::new(EvalMask::new());
    let mut eval_mode: Arc<EvalMode> = Arc::new(EvalMode::NNUE);
    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let mut search_handles: Vec<thread::JoinHandle<()>> = Vec::new();
    let num_threads: AtomicUsize = AtomicUsize::new(1);

    for line in io::stdin().lock().lines() {
        let line: String = line.unwrap();
        let mut tokens: SplitWhitespace<'_> = line.split_whitespace();

        match tokens.next() {
            Some("uci") => {
                println!("id name Vesper {}", VERSION);
                println!("id author Redux");
                println!("option name Threads type spin default 1 min 1 max 16");
                println!("option name Hash type spin default 64 min 1 max 1024");
                println!("option name UseNNUE type check default true");
                println!("uciok");
            }
            Some("setoption") => {
                let rest: Vec<&str> = tokens.collect();
                if let (Some(ni), Some(vi)) = (rest.iter().position(|&t| t == "name"), rest.iter().position(|&t| t == "value")) {
                    let name: String = rest[ni + 1..vi].join(" ");
                    if name.eq_ignore_ascii_case("Threads") {
                        if let Ok(n) = rest[vi + 1..].join(" ").parse::<usize>() {
                            num_threads.store(n.max(1).min(16), Ordering::Relaxed);
                        }
                    }
                    if name.eq_ignore_ascii_case("Hash") {
                        stop_flag.store(true, Ordering::Relaxed);
                        for h in search_handles.drain(..) {
                            let _ = h.join();
                        }
                        stop_flag.store(false, Ordering::Relaxed);

                        if let Ok(n) = rest[vi + 1..].join(" ").parse::<usize>() {
                            tt = Arc::new(TranspositionTable::new(n.max(1).min(1024)));
                        }
                    }
                    if name.eq_ignore_ascii_case("UseNNUE") {
                        let value: String = rest[vi + 1..].join(" ");
                        eval_mode = Arc::new(if value.eq_ignore_ascii_case("true") { EvalMode::NNUE } else { EvalMode::Classical });
                    }
                        
                }
            }
            Some("isready") => {
                println!("readyok");
            }
            Some("ucinewgame") => {
                *board.lock().unwrap() = Board::start_position().unwrap();
                tt.clear();
                *history.lock().unwrap() = vec![board.lock().unwrap().zobrist_key];
            }
            Some("position") => {
                let mut hist: MutexGuard<'_, Vec<u64>> = history.lock().unwrap();
                let new_board: Board = handle_position_command(&mut tokens, &tables, &mut hist);
                *board.lock().unwrap() = new_board;
            }
            Some("go") => {
                stop_flag.store(true, Ordering::Relaxed);
                for h in search_handles.drain(..) {
                    let _ = h.join();
                }
                stop_flag.store(false, Ordering::Relaxed);

                let params: GoParams = parse_go_command(tokens);
                let n: usize = num_threads.load(Ordering::Relaxed).max(1);

                let root_board: Board = board.lock().unwrap().clone();
                let root_history: Vec<u64> = history.lock().unwrap().clone();
                let (soft_deadline, hard_deadline, max_depth) = compute_deadline(&params, root_board.side_to_move);
                let global_nodes: Arc<AtomicU64> = Arc::new(AtomicU64::new(0));

                for thread_id in 0..n {
                    let mut thread_board: Board = root_board.clone();
                    let mut thread_history: Vec<u64> = root_history.clone();
                    let (tables, masks, weights, tt, lmr_table, eval_mode) = (
                        Arc::clone(&tables),
                        Arc::clone(&eval_mask),
                        Arc::clone(&weights),
                        Arc::clone(&tt),
                        Arc::clone(&lmr_table),
                        Arc::clone(&eval_mode),
                    );
                    let stop_flag: Arc<AtomicBool> = Arc::clone(&stop_flag);
                    let global_nodes: Arc<AtomicU64> = Arc::clone(&global_nodes);
                    let is_main: bool = thread_id == 0;
                    let depth_offset: u32 = if is_main { 0 } else { (thread_id as u32) % 4 };

                    search_handles.push(thread::spawn(move || {
                        let mut ctrl: SearchControl = SearchControl::new(soft_deadline, hard_deadline, stop_flag, global_nodes);

                        let on_info = move |info: &SearchInfo| {
                            if is_main {
                                println!(
                                    "info depth {} score {} nodes {} time {} pv {}",
                                    info.depth,
                                    format_score(info.score),
                                    info.nodes,
                                    info.time_ms,
                                    info.pv.iter().map(|&m| move_to_uci_string(m)).collect::<Vec<String>>().join(" ")
                                );
                            }
                        };

                        let best_move: Move = search_best_move(
                            &mut thread_board, &tables, &tt, &weights, &lmr_table, &masks,
                            &mut thread_history, &mut ctrl, max_depth, depth_offset, &eval_mode, on_info
                        );

                        if is_main {
                            println!("bestmove {}", move_to_uci_string(best_move));
                        }
                    }));
                }
            }
            Some("stop") => stop_flag.store(true, Ordering::Relaxed),
            Some("quit") => {
                stop_flag.store(false, Ordering::Relaxed);
                for h in search_handles.drain(..) {
                    let _ = h.join();
                }
                break;
            }
            _ => {}
        }
    }
}