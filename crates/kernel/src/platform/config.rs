// Pinned platform contract shared with the host runner; not generic kernel policy.
pub const RAM_BASE: usize = 0x40000000;
pub const RAM_BYTES: usize = 256 * 1024 * 1024;
pub const PAGE_BYTES: usize = 4096;
pub const PHYSICAL_PAGES: usize = RAM_BYTES / PAGE_BYTES;
pub const RAM_L3_TABLES: usize = PHYSICAL_PAGES / 512;
pub const PHYSICAL_BITMAP_WORDS: usize = PHYSICAL_PAGES / u64::BITS as usize;
pub const DYNAMIC_BASE: usize = 0x80000000;
pub const CONFIGURED_CPUS: usize = 2;
pub const ACTIVE_CPUS: usize = 2;
pub const PHYSICAL_TIMER_IRQ: u32 = 30;
// First EL0 foundation workload: fixed affinity, four independent processes per CPU.
pub const USER_PROCESSES_PER_CPU: usize = 4;
pub const USER_PROCESSES: usize = ACTIVE_CPUS * USER_PROCESSES_PER_CPU;
// Static image/stack budgets for the pinned boot supervisor, not native ABI limits.
pub const USER_PAYLOAD_BASE: usize = 0x2004_0000;
pub const USER_PAYLOAD_BYTES: usize = 128 * 1024;
pub const USER_STACK_PAGES: usize = 4;
