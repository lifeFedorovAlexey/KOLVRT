#![no_std]
#![no_main]
mod ipc_measure;
use ipc_measure::*;
use native_userspace::{CounterRequest, counter_value, native, read16, read32, read64};
native_userspace::entry!();
fn counter(sender: u64, id: u64) -> u64 {
    let (b, n) = native::rpc(sender, id, &CounterRequest::Get.encode()).unwrap();
    counter_value(&b[..n]).unwrap()
}
fn ready(receiver: u64) -> u64 {
    let (t, b, n) = receive(receiver);
    assert_eq!(n, 16);
    assert_eq!(read64(&b, 40), 1);
    t
}
fn saturation(c: Config, sender: u64, feedback: u64, index: usize, records: &mut [Record]) -> u64 {
    let token = ready(feedback);
    let id = index as u64 + 1;
    let mut payload = [0; 8];
    for (i, b) in payload.iter_mut().enumerate() {
        *b = pattern(id, i)
    }
    let start = native::clock_snapshot();
    let (_, receipt) = native::call(
        1,
        sender,
        id,
        start.ticks.checked_add(start.frequency).unwrap(),
        &payload,
    )
    .unwrap();
    let mid = native::clock_snapshot();
    let mut recorder = 0u64;
    if c.mode == 2 {
        unsafe { core::ptr::write_volatile(&mut recorder, mid.ticks) }
    }
    let frequency = reject_sample(c, sender, index + 1, &mut records[1]);
    assert_eq!(records[1][6], (1 << 32) | 12);
    reply(feedback, token, &[1]);
    native::call(5, receipt, 0, 0, &[]).unwrap();
    let (b, _) = native::call(6, receipt, 0, 0, &[]).unwrap();
    let end = native::clock_snapshot();
    assert_eq!(read16(&b, 2), 0);
    assert_eq!(read32(&b, 16), 16);
    assert_eq!(read64(&b, 24), 1);
    let seq = read64(&b, 32);
    assert!(seq > 0);
    assert_eq!(start.frequency, frequency);
    assert_eq!(mid.frequency, frequency);
    assert_eq!(end.frequency, frequency);
    assert!(start.ticks <= mid.ticks && mid.ticks <= end.ticks);
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
    records[0] = [
        u64::from(index >= c.warm),
        id,
        start.ticks,
        end.ticks,
        window,
        read,
        0,
        seq,
    ];

    frequency
}
// Capacity rejection has no receipt/Wait/Collect. Keep its frame separate from
// the retained admitted A request, within the ordinary four-page actor stack.
fn reject_sample(c: Config, sender: u64, index: usize, record: &mut Record) -> u64 {
    let id = index as u64 + 1;
    let mut bytes = [0; 8];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = pattern(id, i)
    }
    let start = native::clock_snapshot();
    let result = native::call(
        1,
        sender,
        id,
        start.ticks.checked_add(start.frequency).unwrap(),
        &bytes,
    );
    let middle = native::clock_snapshot();
    let mut recorder = 0;
    if c.mode == 2 {
        unsafe { core::ptr::write_volatile(&mut recorder, middle.ticks) }
    }
    let end = native::clock_snapshot();
    let status = match result {
        Err(error) => code(error, 1),
        Ok(_) => fail(191),
    };
    assert_eq!(status, (1 << 32) | 12);
    assert_eq!(start.frequency, middle.frequency);
    assert_eq!(start.frequency, end.frequency);
    assert!(start.frequency > 0 && start.ticks <= middle.ticks && middle.ticks <= end.ticks);
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
    *record = [
        u64::from(index >= c.warm),
        id,
        start.ticks,
        end.ticks,
        window,
        read,
        status,
        0,
    ];
    start.frequency
}
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, _feedback: u64, argument: u64) -> ! {
    let c = Config::parse(argument);
    let (service, peer, slot) = setup(c, argument);
    let mut records = [[0u64; 8]; MAX];
    let (frequency, start, end) = measure(c, service, peer, &mut records);
    finish(
        c,
        service,
        slot,
        &records,
        c.count(),
        if c.two() { c.count() } else { 0 },
        frequency,
        start,
        end,
    )
}
fn setup(c: Config, argument: u64) -> ([u64; 5], [u64; 5], usize) {
    let slot = if c.same() { 1 } else { 0 };
    let service = life(1, slot, 0, argument);
    if c.counter() {
        assert_eq!(counter(service[2], 900001), 0)
    }
    let peer = if c.two() {
        let p = life(1, 1, 0, argument);
        let bound = life(6, 1, p[1], service[1]);
        let (t, b, n) = receive(p[3]);
        assert_eq!(n, 8);
        assert_eq!(read64(&b, 40), 1);
        reply(p[3], t, &[bound[1]]);
        p
    } else {
        [0; 5]
    };
    (service, peer, slot)
}
fn measure(
    c: Config,
    service: [u64; 5],
    peer: [u64; 5],
    records: &mut [Record; MAX],
) -> (u64, u64, u64) {
    let mut frequency = 0;
    let mut start = 0;
    let rootcount = c.count();
    if c.case == 9 {
        for i in (0..rootcount).step_by(3) {
            if i == c.warm {
                start = native::clock().0
            }
            let f = saturation(c, service[2], service[3], i, &mut records[i..i + 3]);
            assert_eq!(sample(c, service[2], 0, i + 2, &mut records[i + 2]), f);
            assert_eq!(records[i + 2][6], 0);
            if frequency != 0 {
                assert_eq!(frequency, f)
            }
            frequency = f;
        }
    } else {
        for (i, record) in records.iter_mut().enumerate().take(c.warm_each()) {
            let f = sample(c, service[2], 0, i, record);
            if frequency != 0 {
                assert_eq!(frequency, f)
            }
            frequency = f;
        }
        if c.two() {
            let (t, b, n) = receive(peer[3]);
            assert_eq!(n, 16);
            assert_eq!(read64(&b, 40), 2);
            if frequency != 0 {
                assert_eq!(read64(&b, 48), frequency)
            }
            start = native::clock().0;
            reply(peer[3], t, &[1]);
        } else {
            start = native::clock().0;
        }
        for (i, record) in records
            .iter_mut()
            .enumerate()
            .take(rootcount)
            .skip(c.warm_each())
        {
            let f = sample(c, service[2], 0, i, record);
            if frequency != 0 {
                assert_eq!(frequency, f)
            }
            frequency = f;
        }
    }
    let mut end = native::clock().0;
    let peercount = if c.two() { rootcount } else { 0 };
    if c.two() {
        let (t, b, n) = receive(peer[3]);
        assert_eq!(n, 24);
        assert_eq!(read64(&b, 40), 3);
        assert_eq!(read64(&b, 48), peercount as u64);
        assert_eq!(read64(&b, 56), frequency);
        end = native::clock().0;
        reply(peer[3], t, &[4]);
        let mut at = 0;
        let mut ordinal = 0;
        while at < peercount {
            let (t, b, n) = receive(peer[3]);
            let count = (peercount - at).min(3);
            assert_eq!(n, 32 + count * 64);
            assert_eq!(read64(&b, 40), 4);
            assert_eq!(read64(&b, 48), ordinal);
            assert_eq!(read64(&b, 56), peercount as u64);
            assert_eq!(read64(&b, 64), count as u64);
            for j in 0..count {
                for (k, word) in records[rootcount + at + j].iter_mut().enumerate() {
                    *word = read64(&b, 72 + j * 64 + k * 8)
                }
            }
            reply(peer[3], t, &[ordinal]);
            at += count;
            ordinal += 1;
        }
        let done = completion(1, peer[1]);
        assert_eq!(done[1], 1);
        assert_eq!(done[2], 0);
    }
    (frequency, start, end)
}
#[allow(clippy::too_many_arguments)]
fn finish(
    c: Config,
    service: [u64; 5],
    slot: usize,
    records: &[Record; MAX],
    rootcount: usize,
    peercount: usize,
    frequency: u64,
    start: u64,
    end: u64,
) -> ! {
    let total = rootcount + peercount;
    let mut header = [0u64; 128];
    header[..11].copy_from_slice(&[
        MAGIC,
        1,
        c.case as u64,
        c.mode,
        frequency,
        c.warm as u64,
        c.measured as u64,
        if c.two() { 2 } else { 1 },
        total as u64,
        start,
        end,
    ]);
    header[11] = total as u64;
    let mut digest = 0u64;
    for (i, r) in records[..total].iter().enumerate() {
        let client = u64::from(i >= rootcount);
        let index = if client == 0 { i } else { i - rootcount };
        assert_eq!(r[0], (client << 32) | u64::from(index >= c.warm_each()));
        assert_eq!(r[1], (client << 32) | (index as u64 + 1));
        assert!(r[3] >= r[2]);
        if r[6] >> 32 != 1 {
            header[12] += 1
        }
        if r[6] == 0 {
            header[13] += 1;
            digest = digest.wrapping_add(hash(r[1], r[7]));
            assert!(r[7] > 0);
            for prior in &records[..i] {
                if prior[6] == 0 {
                    assert_ne!(prior[7], r[7])
                }
            }
        } else if r[6] == (1 << 32) | 12 {
            header[14] += 1;
            assert!(c.two() || c.case == 9)
        } else {
            if r[6] & 0xffffffff == 16 {
                header[15] += 1
            } else {
                header[16] += 1
            }
        }
    }
    if c.counter() {
        header[19] = counter(service[2], 900002);

        header[17] = header[19];
        header[18] = digest;
    } else {
        if c.case == 9 {
            let t = ready(service[3]);
            reply(service[3], t, &[1]);
        }
        let (b, n) = rpc_words(service[2], 900003, &[MAGIC, 0, 0]);
        assert_eq!(n, 16);
        header[17] = read64(&b, 0);
        header[18] = read64(&b, 8);
    }
    for r in &records[..total] {
        if r[6] == 0 {
            assert!(r[7] <= header[13])
        }
    }
    header[21] = c.payload() as u64;
    header[22] = u64::from(c.same());
    header[24] = rootcount as u64;
    header[25] = peercount as u64;
    header[26] = c.warm_each() as u64;
    header[27] = if c.two() { c.warm_each() as u64 } else { 0 };
    header[28] = u64::from(
        header[15] == 0 && header[16] == 0 && header[17] == header[13] && header[18] == digest,
    );
    life(5, 0, 0, 0);
    life(4, slot, service[1], 0);
    assert_eq!(completion(slot, service[1])[1], 3);
    for word in &header {
        native::report(*word)
    }
    for r in &records[..total] {
        for word in r {
            native::report(*word)
        }
    }
    native::exit(0)
}
