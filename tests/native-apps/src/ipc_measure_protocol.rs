#![allow(dead_code)]
pub const MAGIC: u64 = 0x4950433301;
pub const MAX: usize = 36;
pub type Record = [u64; 8];
#[derive(Clone, Copy)]
pub struct Config {
    pub case: usize,
    pub mode: u64,
    pub warm: usize,
    pub measured: usize,
}
impl Config {
    pub fn parse(arg: u64) -> Self {
        let c = Self {
            case: (arg & 255) as usize,
            mode: (arg >> 8) & 255,
            warm: ((arg >> 32) & 65535) as usize,
            measured: ((arg >> 16) & 65535) as usize,
        };
        assert!(arg >> 48 == 0 && c.case < 12 && (c.mode == 1 || c.mode == 2));
        assert!(c.warm + c.measured <= MAX && c.measured > 0);
        if c.two() {
            assert!(c.warm.is_multiple_of(2) && c.measured.is_multiple_of(2))
        }
        if c.case == 9 {
            assert!(c.warm.is_multiple_of(3) && c.measured.is_multiple_of(3))
        }
        c
    }
    pub fn two(self) -> bool {
        self.case < 9 && self.case % 3 == 2 || self.case == 11
    }
    pub fn same(self) -> bool {
        self.case < 9 && self.case.is_multiple_of(3)
    }
    pub fn counter(self) -> bool {
        self.case >= 10
    }
    pub fn payload(self) -> usize {
        if self.counter() {
            16
        } else if self.case == 9 {
            8
        } else {
            [0, 8, 256][self.case / 3]
        }
    }
    pub fn count(self) -> usize {
        (self.warm + self.measured) / if self.two() { 2 } else { 1 }
    }
    pub fn warm_each(self) -> usize {
        self.warm / if self.two() { 2 } else { 1 }
    }
}
pub fn pattern(id: u64, i: usize) -> u8 {
    ((id.rotate_right(((i % 8) * 8) as u32) as u8).wrapping_add((i as u8).wrapping_mul(17))) ^ 0x5a
}
pub fn hash(id: u64, seq: u64) -> u64 {
    id.wrapping_add(seq.rotate_left(17))
}
