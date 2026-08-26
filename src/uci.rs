use std::{
    io::{self, BufRead},
    str::SplitWhitespace,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crate::{
    attacks::Tables, bitboard::{Bitboard, Color, PieceType}, board::Board, eval::{EvalMask, Weights}, moves::{Move, MoveFlag, generate_legal_moves}, search::{LMRTable, MATE_THRESHOLD, MATE_VALUE, SearcInfo, SearchControl, search_best_move}, tt::TranspositionTable,
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

struct GoParams {
    depth: Option<u32>,
    movetime: Option<u64>,
    wtime: Option<u64>,
    btime: Option<u64>,
    winc: Option<u64>,
    binc: Option<u64>,
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

fn compute_deadline(p: &GoParams, stm: Color) -> (Instant, Instant, Option<u32>) {
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
    let tt: Arc<Mutex<TranspositionTable>> = Arc::new(Mutex::new(TranspositionTable::new(64))); // 64 MB default
    let board: Arc<Mutex<Board>> = Arc::new(Mutex::new(Board::start_position().unwrap()));
    let weights: Arc<Mutex<Weights>> = Arc::new(Mutex::new(Weights::default()));
    let history: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(vec![board.lock().unwrap().zobrist_key]));
    let lmr_table: Arc<Mutex<LMRTable>> = Arc::new(Mutex::new(LMRTable::new()));
    let eval_mask: Arc<Mutex<EvalMask>> = Arc::new(Mutex::new(EvalMask::new()));
    let stop_flag: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let mut search_handle: Option<thread::JoinHandle<()>> = None;

    for line in io::stdin().lock().lines() {
        let line: String = line.unwrap();
        let mut tokens: SplitWhitespace<'_> = line.split_whitespace();

        match tokens.next() {
            Some("uci") => {
                println!("id name Vesper {}", VERSION);
                println!("id author Redux");
                println!("uciok");
            }
            Some("isready") => {
                println!("readyok");
            }
            Some("ucinewgame") => {
                *board.lock().unwrap() = Board::start_position().unwrap();
                tt.lock().unwrap().clear();
                *history.lock().unwrap() = vec![board.lock().unwrap().zobrist_key];
            }
            Some("position") => {
                let mut hist: MutexGuard<'_, Vec<u64>> = history.lock().unwrap();
                let new_board: Board = handle_position_command(&mut tokens, &tables, &mut hist);
                *board.lock().unwrap() = new_board;
            }
            Some("go") => {
                stop_flag.store(false, Ordering::Relaxed);
                let params: GoParams = parse_go_command(tokens);
                let (board, tables, tt, weights, lmr_table, eval_mask, history, stop_flag) = (
                    Arc::clone(&board),
                    Arc::clone(&tables),
                    Arc::clone(&tt),
                    Arc::clone(&weights),
                    Arc::clone(&lmr_table),
                    Arc::clone(&eval_mask),
                    Arc::clone(&history),
                    Arc::clone(&stop_flag),
                );

                search_handle = Some(thread::spawn(move || {
                    let mut board: MutexGuard<'_, Board> = board.lock().unwrap();
                    let (soft_deadline, hard_deadline, max_depth) = compute_deadline(&params, board.side_to_move);
                    let mut ctrl: SearchControl = SearchControl::new(soft_deadline, hard_deadline, stop_flag);
                    let mut tt: MutexGuard<'_, TranspositionTable> = tt.lock().unwrap();
                    let mut weights: MutexGuard<'_, Weights> = weights.lock().unwrap();
                    let mut history: MutexGuard<'_, Vec<u64>> = history.lock().unwrap();
                    let lmr_table: MutexGuard<'_, LMRTable> = lmr_table.lock().unwrap();
                    let eval_mask: MutexGuard<'_, EvalMask> = eval_mask.lock().unwrap();

                    let best: Move = search_best_move(
                        &mut board,
                        &tables,
                        &mut tt,
                        &mut weights,
                        &lmr_table,
                        &eval_mask,
                        &mut history,
                        &mut ctrl,
                        max_depth,
                        |info: &SearcInfo| {
                            let pv: Vec<String> =
                                info.pv.iter().map(|&mv| move_to_uci_string(mv)).collect();
                            println!(
                                "info depth {} score {} nodes {} time {} pv {}",
                                info.depth,
                                format_score(info.score),
                                info.nodes,
                                info.time_ms,
                                pv.join(" ")
                            );
                        },
                    );
                    println!("bestmove {}", move_to_uci_string(best));
                }));
            }
            Some("stop") => stop_flag.store(true, Ordering::Relaxed),
            Some("quit") => {
                stop_flag.store(false, Ordering::Relaxed);
                if let Some(h) = search_handle.take() {
                    let _ = h.join();
                }
                break;
            }
            _ => {}
        }
    }
}