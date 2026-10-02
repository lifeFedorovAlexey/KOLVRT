//! Current runtime storage bound, not a boot fixture count or stable native ABI.
pub const TASKS_PER_CPU: usize = 4;
pub const TOTAL_TASKS: usize = TASKS_PER_CPU * crate::platform::config::ACTIVE_CPUS;
