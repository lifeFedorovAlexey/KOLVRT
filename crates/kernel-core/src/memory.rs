const BITS_PER_WORD: usize = u64::BITS as usize;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Exhausted,
    Occupied,
}

/// One authoritative pool; reserved and allocated units cannot be allocated again.
pub struct Pool<const N: usize> {
    used: [u64; N],
    units: usize,
    free: usize,
    next: usize,
    #[cfg(test)]
    inspected_units: core::cell::Cell<usize>,
}
impl<const N: usize> Pool<N> {
    pub const fn empty() -> Self {
        Self {
            used: [0; N],
            units: 0,
            free: 0,
            next: 0,
            #[cfg(test)]
            inspected_units: core::cell::Cell::new(0),
        }
    }
    pub fn initialize(&mut self, units: usize) -> Result<(), Error> {
        if units > N * BITS_PER_WORD || self.units != 0 {
            return Err(Error::Invalid);
        }
        self.units = units;
        self.free = units;
        Ok(())
    }
    fn busy(&self, i: usize) -> bool {
        #[cfg(test)]
        self.inspected_units.set(self.inspected_units.get() + 1);
        self.used[i / BITS_PER_WORD] & (1 << (i % BITS_PER_WORD)) != 0
    }
    fn set(&mut self, i: usize, value: bool) {
        if self.busy(i) != value {
            if value {
                self.free -= 1;
            } else {
                self.free += 1;
            }
        }
        if value {
            self.used[i / BITS_PER_WORD] |= 1 << (i % BITS_PER_WORD);
        } else {
            self.used[i / BITS_PER_WORD] &= !(1 << (i % BITS_PER_WORD));
        }
    }
    pub fn reserve(&mut self, start: usize, count: usize) -> Result<(), Error> {
        let end = start.checked_add(count).ok_or(Error::Invalid)?;
        if end > self.units {
            return Err(Error::Invalid);
        }
        for i in start..end {
            self.set(i, true);
        }
        Ok(())
    }
    pub fn allocate(&mut self, count: usize, align: usize) -> Result<usize, Error> {
        if count == 0 || !align.is_power_of_two() {
            return Err(Error::Invalid);
        }
        if count > self.free {
            return Err(Error::Exhausted);
        }
        let mut start: usize = if count == 1 {
            self.next.checked_add(align - 1).ok_or(Error::Invalid)? & !(align - 1)
        } else {
            0
        };
        while let Some(end) = start.checked_add(count) {
            if end > self.units {
                break;
            }
            if let Some(busy) = (start..end).find(|&i| self.busy(i)) {
                start = (busy + 1).checked_add(align - 1).ok_or(Error::Invalid)? & !(align - 1);
            } else {
                self.reserve(start, count)?;
                if count == 1 && align == 1 {
                    self.next = end;
                }
                return Ok(start);
            }
        }
        Err(Error::Exhausted)
    }
    /// Claim an exact free extent when a physical address is part of a retained
    /// translation witness. The caller receives the same ownership as allocate.
    pub fn allocate_at(&mut self, start: usize, count: usize) -> Result<(), Error> {
        let end = start.checked_add(count).ok_or(Error::Invalid)?;
        if count == 0 || end > self.units {
            return Err(Error::Invalid);
        }
        if (start..end).any(|i| self.busy(i)) {
            return Err(Error::Occupied);
        }
        self.reserve(start, count)
    }
    /// Caller owns the exact live range; bounds and occupancy are always checked.
    pub fn release(&mut self, start: usize, count: usize) -> Result<(), Error> {
        let end = start.checked_add(count).ok_or(Error::Invalid)?;
        if count == 0 || end > self.units {
            return Err(Error::Invalid);
        }
        if (start..end).any(|i| !self.busy(i)) {
            return Err(Error::Invalid);
        }
        for i in start..end {
            self.set(i, false);
        }
        self.next = self.next.min(start);
        Ok(())
    }
    pub fn available(&self) -> usize {
        self.free
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_preserves_ownership_and_linear_scan_work() {
        const UNITS: usize = 256;
        let mut candidate = Pool::<4>::empty();
        let mut rescan_reference = Pool::<4>::empty();
        candidate.initialize(UNITS).unwrap();
        rescan_reference.initialize(UNITS).unwrap();
        for expected in 0..UNITS {
            // Test-only counterfactual: same allocator, cursor optimization disabled.
            rescan_reference.next = 0;
            assert_eq!(candidate.allocate(1, 1), Ok(expected));
            assert_eq!(rescan_reference.allocate(1, 1), Ok(expected));
            assert_eq!(candidate.available(), rescan_reference.available());
        }
        let fast_work = candidate.inspected_units.get();
        let rescan_work = rescan_reference.inspected_units.get();
        assert_eq!(fast_work, 2 * UNITS); // One search and one ownership update per unit.
        assert_eq!(rescan_work, fast_work + UNITS * (UNITS - 1) / 2);
        let before_exhaustion = candidate.inspected_units.get();
        assert_eq!(candidate.allocate(1, 1), Err(Error::Exhausted));
        assert_eq!(candidate.inspected_units.get(), before_exhaustion);
        for unit in 0..UNITS {
            candidate.release(unit, 1).unwrap();
            rescan_reference.release(unit, 1).unwrap();
        }
        assert_eq!(candidate.available(), UNITS);
        assert_eq!(rescan_reference.available(), UNITS);
    }
    #[test]
    fn exact_extent_claim_preserves_pool_ownership() {
        let mut pool = Pool::<1>::empty();
        pool.initialize(64).unwrap();
        pool.reserve(8, 4).unwrap();
        assert_eq!(pool.allocate_at(8, 4), Err(Error::Occupied));
        pool.allocate_at(16, 8).unwrap();
        assert_eq!(pool.available(), 52);
        pool.release(16, 8).unwrap();
        assert_eq!(pool.available(), 60);
        assert_eq!(pool.allocate_at(64, 1), Err(Error::Invalid));
    }
    #[test]
    fn exhaustion_reuse_and_transactional_rejection() {
        let mut pool = Pool::<1>::empty();
        pool.initialize(4).unwrap();
        pool.reserve(0, 1).unwrap();
        assert_eq!(pool.allocate(3, 1), Ok(1));
        assert_eq!(pool.allocate(1, 1), Err(Error::Exhausted));
        assert_eq!(pool.release(3, 2), Err(Error::Invalid));
        assert_eq!(pool.available(), 0);
        pool.release(1, 3).unwrap();
        assert_eq!(pool.allocate(1, 2), Ok(2));
        assert!(pool.allocate(1, 3).is_err());
    }
}
