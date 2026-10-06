#![no_std]

pub const FRAME_BYTES: usize = 296;
pub const HEADER_BYTES: usize = 40;
pub const REPORT_MAGIC: u64 = 0x4b4f_4c56_5254_3701;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid,
    Overflow,
    Native(u64),
    Outcome(u16),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CounterRequest {
    Get,
    Add(u64),
}
impl CounterRequest {
    pub fn encode(self) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[..2].copy_from_slice(&1u16.to_le_bytes());
        let (operation, delta) = match self {
            Self::Get => (1u16, 0u64),
            Self::Add(delta) => (2, delta),
        };
        bytes[2..4].copy_from_slice(&operation.to_le_bytes());
        bytes[8..].copy_from_slice(&delta.to_le_bytes());
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != 16 || read16(bytes, 0) != 1 || read32(bytes, 4) != 0 {
            return Err(Error::Invalid);
        }
        match (read16(bytes, 2), read64(bytes, 8)) {
            (1, 0) => Ok(Self::Get),
            (2, delta) => Ok(Self::Add(delta)),
            _ => Err(Error::Invalid),
        }
    }
}

/// State is stored through its private userspace address, never a kernel helper.
pub struct Counter {
    value: u64,
}
impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}
impl Counter {
    pub const fn new() -> Self {
        Self { value: 0 }
    }
    pub fn value(&self) -> u64 {
        // SAFETY: this initialized cell belongs to this process and immutable borrow.
        unsafe { core::ptr::read_volatile(&self.value) }
    }
    pub fn apply(&mut self, request: CounterRequest) -> Result<u64, Error> {
        let old = self.value();
        let value = match request {
            CounterRequest::Get => old,
            CounterRequest::Add(delta) => old.checked_add(delta).ok_or(Error::Overflow)?,
        };
        // SAFETY: exclusive initialized private cell; no shared process mapping.
        unsafe {
            core::ptr::write_volatile(&mut self.value, value);
        }
        Ok(value)
    }
}

pub fn counter_reply(result: Result<u64, Error>) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[..2].copy_from_slice(&1u16.to_le_bytes());
    let (status, value) = match result {
        Ok(value) => (0u16, value),
        Err(Error::Overflow) => (2, 0),
        Err(_) => (1, 0),
    };
    bytes[2..4].copy_from_slice(&status.to_le_bytes());
    bytes[8..].copy_from_slice(&value.to_le_bytes());
    bytes
}
pub fn counter_value(bytes: &[u8]) -> Result<u64, Error> {
    if bytes.len() != 16 || read16(bytes, 0) != 1 || read32(bytes, 4) != 0 {
        return Err(Error::Invalid);
    }
    match read16(bytes, 2) {
        0 => Ok(read64(bytes, 8)),
        2 => Err(Error::Overflow),
        _ => Err(Error::Invalid),
    }
}
pub fn read16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}
pub fn read32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
pub fn read64(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}
pub fn frame(
    operation: u16,
    selector: u64,
    id: u64,
    deadline: u64,
    payload: &[u8],
) -> ([u8; FRAME_BYTES], usize) {
    assert!(payload.len() <= 256);
    let mut bytes = [0; FRAME_BYTES];
    let length = HEADER_BYTES + payload.len();
    bytes[..2].copy_from_slice(&1u16.to_le_bytes());
    bytes[2..4].copy_from_slice(&operation.to_le_bytes());
    bytes[4..8].copy_from_slice(&(length as u32).to_le_bytes());
    bytes[8..16].copy_from_slice(&selector.to_le_bytes());
    bytes[16..24].copy_from_slice(&id.to_le_bytes());
    bytes[24..32].copy_from_slice(&deadline.to_le_bytes());
    bytes[32..36].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes[HEADER_BYTES..length].copy_from_slice(payload);
    (bytes, length)
}

#[cfg(target_arch = "aarch64")]
pub mod native {
    use super::*;
    pub fn ipc(input: &[u8], output: &mut [u8; FRAME_BYTES]) -> (u64, u64) {
        let status: u64;
        let token: u64;
        // SAFETY: initialized caller-owned slices remain live for the synchronous native ABI;
        // memory is clobbered, all returned registers are declared, no Rust ABI crosses EL1.
        unsafe {
            core::arch::asm!("svc #0xa0",inlateout("x0") input.as_ptr() as u64=>status,inlateout("x1") input.len() as u64=>token,inlateout("x2") output.as_mut_ptr() as u64=>_,inlateout("x3") FRAME_BYTES as u64=>_,options(nostack));
        }
        (status, token)
    }
    pub fn lifecycle(operation: u64, selector: u64, token: u64, argument: u64) -> [u64; 5] {
        let mut words = [operation, selector, token, argument, 1];
        // SAFETY: provisional initialized native.lifecycle/1 register protocol only.
        unsafe {
            core::arch::asm!("svc #0xb0",inlateout("x0") words[0],inlateout("x1") words[1],inlateout("x2") words[2],inlateout("x3") words[3],inlateout("x4") words[4],options(nostack));
        }
        words
    }
    pub fn clock() -> (u64, u64) {
        let ticks: u64;
        let frequency: u64;
        // SAFETY: read-only public native clock ABI; every changed register is declared.
        unsafe {
            core::arch::asm!("svc #0x53",lateout("x0") ticks,lateout("x1") frequency,lateout("x2") _,lateout("x3") _,lateout("x4") _,options(nostack));
        }
        (ticks, frequency)
    }
    pub fn report(word: u64) {
        // SAFETY: bounded public diagnostic word channel; it confers no authority.
        unsafe {
            core::arch::asm!("svc #0x54",inlateout("x0") word=>_,options(nostack));
        }
    }
    pub fn exit(code: u64) -> ! {
        // SAFETY: public own-process terminal call; no stack or resource is reclaimed here.
        unsafe {
            core::arch::asm!("svc #0x4e",in("x0") code,options(noreturn));
        }
    }
    pub fn fault() -> ! {
        // SAFETY: deliberate isolated EL0 illegal instruction used for fault containment.
        unsafe {
            core::arch::asm!(".inst 0", options(noreturn));
        }
    }
    pub fn call(
        operation: u16,
        selector: u64,
        id: u64,
        deadline: u64,
        payload: &[u8],
    ) -> Result<([u8; FRAME_BYTES], u64), Error> {
        let (input, length) = frame(operation, selector, id, deadline, payload);
        let mut output = [0; FRAME_BYTES];
        let (status, token) = ipc(&input[..length], &mut output);
        if status != 0 {
            Err(Error::Native(status))
        } else {
            Ok((output, token))
        }
    }
    pub fn rpc(handle: u64, id: u64, payload: &[u8]) -> Result<([u8; 256], usize), Error> {
        let (ticks, frequency) = clock();
        let deadline = ticks.checked_add(frequency).ok_or(Error::Invalid)?;
        let (_, receipt) = call(1, handle, id, deadline, payload)?;
        call(5, receipt, 0, 0, &[])?;
        let (output, _) = call(6, receipt, 0, 0, &[])?;
        if read16(&output, 0) != 1 {
            return Err(Error::Invalid);
        }
        let outcome = read16(&output, 2);
        if outcome != 0 {
            return Err(Error::Outcome(outcome));
        }
        let length = read32(&output, 16) as usize;
        if length > 256 || read32(&output, 4) as usize != 24 + length {
            return Err(Error::Invalid);
        }
        let mut bytes = [0; 256];
        bytes[..length].copy_from_slice(&output[24..24 + length]);
        Ok((bytes, length))
    }
}

#[macro_export]
macro_rules! entry {
    () => {
        core::arch::global_asm!(".section .text.entry,\"ax\"\n.global _start\n_start:\nmov x0,x21\nmov x1,x22\nmov x2,x23\nbl native_main\nb .");
        #[panic_handler]
        fn panic(_: &core::panic::PanicInfo<'_>)->! {$crate::native::exit(255)}
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_state_and_restart_initial_value() {
        let mut state = Counter::new();
        assert_eq!(state.apply(CounterRequest::Add(5)), Ok(5));
        assert_eq!(state.apply(CounterRequest::Add(7)), Ok(12));
        assert_eq!(state.apply(CounterRequest::Get), Ok(12));
        assert_eq!(Counter::new().value(), 0);
    }
    #[test]
    fn overflow_does_not_mutate_state() {
        let mut state = Counter::new();
        state.apply(CounterRequest::Add(u64::MAX)).unwrap();
        assert_eq!(state.apply(CounterRequest::Add(1)), Err(Error::Overflow));
        assert_eq!(state.value(), u64::MAX);
    }
    #[test]
    fn parser_rejects_reserved_fields_versions_lengths_and_get_delta() {
        for request in [CounterRequest::Get, CounterRequest::Add(u64::MAX)] {
            assert_eq!(CounterRequest::decode(&request.encode()), Ok(request));
        }
        let good = CounterRequest::Get.encode();
        for n in 0..16 {
            assert_eq!(CounterRequest::decode(&good[..n]), Err(Error::Invalid));
        }
        for at in [0, 2, 4, 8] {
            let mut bad = good;
            bad[at] = 99;
            assert_eq!(CounterRequest::decode(&bad), Err(Error::Invalid));
        }
    }
    #[test]
    fn native_frames_are_explicit_initialized_little_endian() {
        let payload = CounterRequest::Add(7).encode();
        let (bytes, n) = frame(1, 9, 11, 13, &payload);
        assert_eq!(n, 56);
        assert_eq!(read16(&bytes, 0), 1);
        assert_eq!(read64(&bytes, 8), 9);
        assert_eq!(read64(&bytes, 16), 11);
        assert_eq!(read64(&bytes, 24), 13);
        assert_eq!(read32(&bytes, 32), 16);
        assert!(bytes[n..].iter().all(|b| *b == 0));
        assert_eq!(counter_value(&counter_reply(Ok(12))), Ok(12));
    }
}
