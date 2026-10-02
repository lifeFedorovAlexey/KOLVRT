//! Native own-task observation boundary. Caller attribution is established by the
//! scheduler's protected current-task lookup; no caller ID or user pointer is accepted.
use crate::cpu;
use kernel_core::{execution as abi, window};
#[derive(Clone, Copy)]
pub struct Observations {
    history: [u32; abi::HISTORY_WORDS],
    captured: Option<[u32; abi::HISTORY_WORDS]>,
    last_tick: u64,
    #[cfg(feature = "machine-events")]
    native_attempts: u64,
}
impl Observations {
    pub const ZERO: Self = Self {
        history: [0; abi::HISTORY_WORDS],
        captured: None,
        last_tick: 0,
        #[cfg(feature = "machine-events")]
        native_attempts: 0,
    };
    // Indexed CPU only with IRQ masked; immutable capture never changes after admission.
    pub fn service_timer(&mut self, slice: usize) {
        let now = cpu::ticks();
        if self.last_tick != 0 {
            self.history[(slice - 1) % abi::HISTORY_WORDS] =
                now.saturating_sub(self.last_tick).min(u32::MAX as u64) as u32;
        }
        self.last_tick = now;
    }
    pub fn call(&mut self, operation: u16, registers: &mut [u64]) -> bool {
        match operation {
            abi::CAPTURE => {
                if self.captured.is_none() {
                    self.captured = Some(self.history);
                }
                registers[0] = abi::OK;
                registers[1] = abi::HISTORY_WORDS as u64;
            }
            abi::READ_WINDOW => {
                #[cfg(feature = "machine-events")]
                {
                    self.native_attempts = self.native_attempts.saturating_add(1);
                }
                let start = u32::try_from(registers[0]);
                let end = u32::try_from(registers[1]);
                // All public output registers initialized before either success or error.
                registers[..abi::REPLY_REGISTERS].fill(0);
                registers[0] = abi::INVALID;
                if let (Ok(start), Ok(end)) = (start, end) {
                    if let Some(data) = &self.captured {
                        if let Ok(words) = window::checked(data, window::Span { start, end }) {
                            registers[0] = abi::OK;
                            registers[1] = words.len() as u64;
                            for (index, &value) in words.iter().enumerate() {
                                registers[abi::DATA_REGISTER_START
                                    + index / abi::VALUES_PER_REGISTER] |= u64::from(value)
                                    << ((index % abi::VALUES_PER_REGISTER) * u32::BITS as usize);
                            }
                        }
                    } else {
                        registers[0] = abi::NOT_CAPTURED;
                    }
                }
            }
            _ => return false,
        }
        true
    }
    #[cfg(all(feature = "machine-events", feature = "boot-payload"))]
    pub fn attempts(&self) -> u64 {
        self.native_attempts
    }
}
