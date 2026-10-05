//! Fixed identity storage. A retained lease excludes cell reuse without heap allocation.
use super::{DomainId, Limits};
use crate::process::ProcessId;
use core::{
    ops::Deref,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};
const SLOTS: usize = 32;
const RESERVED: usize = usize::MAX;
const MAX_REFERENCES: usize = 1024;
pub(super) struct Cell {
    generation: AtomicU64,
    references: AtomicUsize,
    pub(super) state: AtomicU64,
    pub(super) pages: AtomicUsize,
}
impl Cell {
    const fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            references: AtomicUsize::new(0),
            state: AtomicU64::new(0),
            pages: AtomicUsize::new(0),
        }
    }
}
static CELLS: [Cell; SLOTS] = [const { Cell::new() }; SLOTS];
pub(super) struct Lease {
    pub(super) id: DomainId,
    pub(super) limits: Limits,
}
impl Lease {
    pub(super) fn new(process: ProcessId, limits: Limits, pages: usize) -> Option<Self> {
        for (index, cell) in CELLS.iter().enumerate() {
            if cell
                .references
                .compare_exchange(0, RESERVED, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            let Some(generation) = cell.generation.load(Ordering::Relaxed).checked_add(1) else {
                continue;
            };
            cell.state.store(0, Ordering::Relaxed);
            cell.pages.store(pages, Ordering::Relaxed);
            cell.generation.store(generation, Ordering::Release);
            cell.references.store(1, Ordering::Release);
            return Some(Self {
                id: DomainId(process, index as u16, generation),
                limits,
            });
        }
        None
    }
}
impl Deref for Lease {
    type Target = Cell;
    fn deref(&self) -> &Cell {
        let cell = &CELLS[self.id.1 as usize];
        assert_eq!(cell.generation.load(Ordering::Acquire), self.id.2);
        assert_ne!(cell.references.load(Ordering::Acquire), 0);
        cell
    }
}
impl Clone for Lease {
    fn clone(&self) -> Self {
        let cell = self.deref();
        let mut refs = cell.references.load(Ordering::Acquire);
        loop {
            assert!(
                refs > 0 && refs < MAX_REFERENCES,
                "trusted domain reference bound"
            );
            match cell.references.compare_exchange_weak(
                refs,
                refs + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Self {
                        id: self.id,
                        limits: self.limits,
                    };
                }
                Err(next) => refs = next,
            }
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        let previous = self.references.fetch_sub(1, Ordering::AcqRel);
        assert!(previous > 0 && previous != RESERVED);
        // No access after the final release: another creator can now reuse this cell.
    }
}
pub(super) fn live() -> usize {
    CELLS
        .iter()
        .filter(|cell| {
            let n = cell.references.load(Ordering::Acquire);
            n != 0 && n != RESERVED
        })
        .count()
}
