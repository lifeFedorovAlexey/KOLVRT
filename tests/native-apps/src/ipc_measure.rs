#![allow(dead_code)]
use native_userspace::{CounterRequest, Error, counter_value, native, read16, read32, read64};
#[path = "ipc_measure_protocol.rs"]
mod protocol;
pub use protocol::*;
pub fn fail(code: u64) -> ! {
    for w in [MAGIC, 0, code] {
        native::report(w)
    }
    native::exit(code)
}
pub fn life(op: u64, slot: usize, token: u64, arg: u64) -> [u64; 5] {
    let r = native::lifecycle(op, slot as u64, token, arg);
    if r[0] != 0 {
        fail(100 + r[0])
    }
    r
}
pub fn completion(slot: usize, token: u64) -> [u64; 5] {
    loop {
        let r = life(2, slot, token, 0);
        if r[1] != 0 {
            return r;
        }
    }
}
pub fn encode(words: &[u64], out: &mut [u8; 256]) -> usize {
    assert!(words.len() <= 32);
    for (i, w) in words.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&w.to_le_bytes())
    }
    words.len() * 8
}
pub fn rpc_words(handle: u64, id: u64, words: &[u64]) -> ([u8; 256], usize) {
    let mut b = [0; 256];
    let n = encode(words, &mut b);
    native::rpc(handle, id, &b[..n]).unwrap_or_else(|_| fail(180))
}
pub fn receive(receiver: u64) -> (u64, [u8; 296], usize) {
    let (b, _) = native::call(2, receiver, 0, 0, &[]).unwrap_or_else(|_| fail(181));
    let token = read64(&b, 8);
    let n = read32(&b, 32) as usize;
    assert!(n <= 256);
    native::call(3, receiver, token, 0, &[]).unwrap_or_else(|_| fail(182));
    (token, b, n)
}
pub fn reply(receiver: u64, token: u64, words: &[u64]) {
    let mut b = [0; 256];
    let n = encode(words, &mut b);
    native::call(4, receiver, token, 0, &b[..n]).unwrap_or_else(|_| fail(183));
}
pub fn code(e: Error, stage: u64) -> u64 {
    match e {
        Error::Native(n) => (stage << 32) | n,
        _ => (stage << 32) | 0xffff,
    }
}
pub fn sample(c: Config, handle: u64, client: u64, index: usize, record: &mut Record) -> u64 {
    let id = (client << 32) | (index as u64 + 1);
    let mut payload = [0u8; 256];
    let n = c.payload();
    if c.counter() {
        payload[..16].copy_from_slice(&CounterRequest::Add(1).encode())
    } else {
        for (i, b) in payload[..n].iter_mut().enumerate() {
            *b = pattern(id, i)
        }
    }
    let start = native::clock_snapshot();
    let deadline = start.ticks.checked_add(start.frequency).unwrap();
    let submit = native::call(1, handle, id, deadline, &payload[..n]);
    let middle = native::clock_snapshot();
    let mut recorder = 0u64;
    if c.mode == 2 {
        unsafe { core::ptr::write_volatile(&mut recorder, middle.ticks) }
    }
    let mut status = 0;
    let mut sequence = 0;
    let mut result = [0u8; 296];
    let mut have = false;
    match submit {
        Err(e) => status = code(e, 1),
        Ok((_, receipt)) => match native::call(5, receipt, 0, 0, &[]) {
            Err(e) => status = code(e, 2),
            Ok(_) => match native::call(6, receipt, 0, 0, &[]) {
                Err(e) => status = code(e, 3),
                Ok((b, _)) => {
                    result = b;
                    have = true
                }
            },
        },
    }
    let end = native::clock_snapshot();
    assert!(
        start.frequency > 0
            && middle.frequency == start.frequency
            && end.frequency == start.frequency
    );
    assert!(start.ticks <= middle.ticks && middle.ticks <= end.ticks);
    let window = end
        .execution_window_ticks
        .checked_sub(start.execution_window_ticks)
        .unwrap();
    let read = end
        .read_window_service_ticks
        .checked_sub(start.read_window_service_ticks)
        .unwrap();
    assert_eq!(read, 0);
    assert!((end.ticks - start.ticks).checked_sub(window).is_some());
    if have {
        assert_eq!(read16(&result, 0), 1);
        let outcome = read16(&result, 2);
        if outcome != 0 {
            status = (4 << 32) | u64::from(outcome)
        } else {
            assert_eq!(read32(&result, 16), 16);
            assert_eq!(read32(&result, 4), 40);
            sequence = if c.counter() {
                counter_value(&result[24..40]).unwrap()
            } else {
                assert_eq!(read64(&result, 24), 1);
                read64(&result, 32)
            };
            assert!(sequence > 0)
        }
    }
    *record = [
        (client << 32) | u64::from(index >= c.warm_each()),
        id,
        start.ticks,
        end.ticks,
        window,
        read,
        status,
        sequence,
    ];
    start.frequency
}
