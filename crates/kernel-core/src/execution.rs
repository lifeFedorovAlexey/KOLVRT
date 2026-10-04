//! Experimental native execution-accounting boundary, not a stable userspace ABI.
//! Every lookup is implicit in the currently executing task; no caller-selected owner.
pub const CAPTURE: u16 = 0x50;
pub const READ_WINDOW: u16 = 0x51;
pub const SLICES: u16 = 0x52;
pub const CLOCK: u16 = 0x53;
pub const REPORT: u16 = 0x54;
pub const FINISH: u16 = 0x4e;
pub const HISTORY_WORDS: usize = 8;
pub const VALUES_PER_REGISTER: usize = u64::BITS as usize / u32::BITS as usize;
pub const DATA_REGISTERS: usize = HISTORY_WORDS.div_ceil(VALUES_PER_REGISTER);
pub const DATA_REGISTER_START: usize = 2; // x0 status, x1 returned word count.
pub const REPLY_REGISTERS: usize = DATA_REGISTER_START + DATA_REGISTERS;
pub const REPORT_WORDS: usize = 2048;
pub const OK: u64 = 0;
pub const INVALID: u64 = 1;
pub const NOT_CAPTURED: u64 = 2;
pub const FULL: u64 = 3;
