use crate::moves::Move;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
pub(crate) struct TTEntry {
    key: u64,
    depth: u8,
    score: i32,
    best_move: Move,
    bound: Bound,
}

impl TTEntry {
    #[inline]
    pub fn _key(&self) -> u64 {
        self.key
    }
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

pub struct TranspositionTable {
    entries: Vec<Option<TTEntry>>,
    mask: usize,
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let count: usize = (size_mb * 1024 * 1024) / std::mem::size_of::<TTEntry>();
        let capacity: usize = count.next_power_of_two() / 2;

        Self {
            entries: vec![None; capacity],
            mask: capacity - 1,
        }
    }
    pub fn clear(&mut self) {
        self.entries
            .iter_mut()
            .for_each(|e: &mut Option<TTEntry>| *e = None);
    }

    pub(crate) fn probe(&self, key: u64) -> Option<TTEntry> {
        self.entries[(key as usize) & self.mask].filter(|e: &TTEntry| e.key == key)
    }
    pub(crate) fn store(&mut self, key: u64, depth: u8, score: i32, best_move: Move, bound: Bound) {
        self.entries[(key as usize) & self.mask] = Some(TTEntry {
            key,
            depth,
            score,
            best_move,
            bound,
        })
    }
}
