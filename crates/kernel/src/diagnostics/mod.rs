use crate::hal::Registers;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicUsize, Ordering};
const UART_REGISTER_SIZE: usize = 0x1000;
const UART_DR: usize = 0x00;
const UART_FR: usize = 0x18;
const UART_FR_TXFF: u32 = 1 << 5;
// Bounded register reads, including when the timer is unavailable.
const UART_TX_POLL_LIMIT: usize = 1_000_000;
static UART: AtomicUsize = AtomicUsize::new(0);
static DROPPED: AtomicUsize = AtomicUsize::new(0);
// Includes event framing and newline. No allocation or unbounded UART wait.
const RECORD_BYTES: usize = 4096;
pub fn initialize(base: usize) {
    UART.store(base, Ordering::Release);
}
pub fn boot_banner() {
    // Fixed ASCII asset: no image decoding, formatting allocation or profile divergence.
    print(format_args!(
        "{}",
        include_str!("../../../../assets/branding/boot-logo.txt")
    ));
}
struct Length(usize);
impl Write for Length {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.checked_add(s.len()).ok_or(fmt::Error)?;
        if self.0 > RECORD_BYTES {
            return Err(fmt::Error);
        }
        Ok(())
    }
}
struct Writer;
impl Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let base = UART.load(Ordering::Acquire);
        if base == 0 {
            return Err(fmt::Error);
        }
        // SAFETY: INV-UART: published only after DT validation; sole CPU polled writer, no IRQ logging.
        let uart = unsafe { Registers::new(base, UART_REGISTER_SIZE) };
        for b in s.bytes() {
            let mut budget = UART_TX_POLL_LIMIT;
            while uart.read(UART_FR) & UART_FR_TXFF != 0 {
                budget -= 1;
                if budget == 0 {
                    return Err(fmt::Error);
                }
            }
            uart.write(UART_DR, u32::from(b));
        }
        Ok(())
    }
}
pub fn print(args: fmt::Arguments<'_>) {
    // Never recursively panic for misuse or unavailable output. CPU1 reports via SMP.
    if crate::percpu::is_secondary() || Length(0).write_fmt(args).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    if Writer.write_fmt(args).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}
pub fn status(level: &str, component: &str, args: fmt::Arguments<'_>) {
    print(format_args!("[{}] {}: {}\n", level, component, args));
}
pub fn dropped() -> usize {
    DROPPED.load(Ordering::Relaxed)
}
#[cfg(feature = "machine-events")]
pub fn event(args: fmt::Arguments<'_>) {
    print(format_args!("@KOLVRT/1 {}\n", args));
}
#[macro_export]
macro_rules! log { ($($arg:tt)*) => { $crate::diagnostics::print(format_args!($($arg)*)) }; }
#[macro_export]
macro_rules! event {
    ($($arg:tt)*) => {{
        #[cfg(feature = "machine-events")]
        $crate::diagnostics::event(format_args!($($arg)*));
        #[cfg(not(feature = "machine-events"))]
        let _ = format_args!($($arg)*);
    }};
}
