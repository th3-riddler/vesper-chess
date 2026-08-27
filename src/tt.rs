use std::sync::atomic::{AtomicU64, Ordering};

use crate::moves::Move;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
pub struct TTEntry {
    depth: u8,
    score: i32,
    best_move: Move,
    bound: Bound,
}

impl TTEntry {
    #[inline]
    pub fn depth(&self) -> u8 {
        self.depth
    }
    #[inline]
    pub fn score(&self) -> i32 {
        self.score
    }
    #[inline]
    pub fn best_move(&self) -> Move {
        self.best_move
    }
    #[inline]
    pub fn bound(&self) -> Bound {
        self.bound
    }
}

struct TTSlot {
    key: AtomicU64, // stores (zobrist_key ^ data)
    data: AtomicU64,
}

pub struct TranspositionTable {
    entries: Vec<TTSlot>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let count: usize = (size_mb * 1024 * 1024) / std::mem::size_of::<TTSlot>();
        let capacity: usize = count.next_power_of_two() / 2;
        let entries: Vec<TTSlot> = (0..capacity).map(|_| TTSlot { key: AtomicU64::new(0), data: AtomicU64::new(0) }).collect();

        Self { entries, mask: capacity - 1 }
    }

    pub fn clear(&self) {
        for slot in &self.entries {
            slot.key.store(0, Ordering::Relaxed);
            slot.data.store(0, Ordering::Relaxed);
        }
    }

    pub fn probe(&self, zobrist_key: u64) -> Option<TTEntry> {
        let slot: &TTSlot = &self.entries[(zobrist_key as usize) & self.mask];
        let data: u64 = slot.data.load(Ordering::Relaxed);
        let key_field = slot.key.load(Ordering::Relaxed);
        
        if key_field ^ data != zobrist_key {
            return None;
        }
        let (depth, score, best_move, bound) = unpack_entry(data);

        Some(TTEntry { depth, score, best_move, bound })
    }

    pub fn store(&self, zobrist_key: u64, depth: u8, score: i32, best_move: Move, bound: Bound) {
        let data = pack_entry(depth, score, best_move, bound);
        let slot = &self.entries[(zobrist_key as usize) & self.mask];
        slot.data.store(data, Ordering::Relaxed);
        slot.key.store(zobrist_key ^ data, Ordering::Relaxed);
    }
}

fn pack_entry(depth: u8, score: i32, best_move: Move, bound: Bound) -> u64 {
    let move_bits: u64 = best_move.0 as u64;
    let score_bits: u64 = (score as i16 as u16) as u64;
    let depth_bits: u64 = depth as u64;
    let bound_bits = match bound {
        Bound::Exact => 0u64,
        Bound::Lower => 1,
        Bound::Upper => 2
    };

    move_bits | (score_bits << 16) | (depth_bits << 32) | (bound_bits << 40)
}

fn unpack_entry(data: u64) -> (u8, i32, Move, Bound) {
    let best_move: Move = Move((data & 0xFFFF) as u16);
    let score: i32 = (((data >> 16) & 0xFFFF) as u16) as i16 as i32;
    let depth = ((data >> 32) & 0xFF) as u8;
    let bound = match (data >> 40) & 0b11 {
        0 => Bound::Exact,
        1 => Bound::Lower,
        _ => Bound::Upper
    };

    (depth, score, best_move, bound)
}