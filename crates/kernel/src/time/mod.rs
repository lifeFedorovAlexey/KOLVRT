use crate::arch::aarch64 as cpu;
pub use core::time::Duration;
pub fn deadline_after(duration: Duration) -> u64 {
    let duration = kernel_core::time::duration_ticks(duration, cpu::frequency())
        .expect("invalid counter duration");
    cpu::ticks()
        .checked_add(duration)
        .expect("counter epoch exhausted")
}
