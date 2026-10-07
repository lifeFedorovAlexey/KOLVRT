#![no_std]
#![no_main]
mod ipc_measure;
use ipc_measure::*;
use native_userspace::{native, read64};
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, feedback: u64, argument: u64) -> ! {
    let c = Config::parse(argument);
    assert!(c.two());
    let mut records = [[0u64; 8]; MAX];
    let (r, n) = rpc_words(feedback, 1, &[1]);
    assert_eq!(n, 8);
    let sender = read64(&r, 0);
    let mut frequency = 0;
    for (i, record) in records.iter_mut().enumerate().take(c.warm_each()) {
        frequency = sample(c, sender, 1, i, record)
    }
    let (r, n) = rpc_words(feedback, 2, &[2, frequency]);
    assert_eq!(n, 8);
    assert_eq!(read64(&r, 0), 1);
    for (i, record) in records
        .iter_mut()
        .enumerate()
        .take(c.count())
        .skip(c.warm_each())
    {
        let f = sample(c, sender, 1, i, record);
        if frequency != 0 {
            assert_eq!(frequency, f)
        }
        frequency = f;
    }
    let (r, n) = rpc_words(feedback, 3, &[3, c.count() as u64, frequency]);
    assert_eq!(n, 8);
    assert_eq!(read64(&r, 0), 4);
    for (start, chunk) in records[..c.count()].chunks(3).enumerate() {
        let mut words = [0u64; 28];
        words[..4].copy_from_slice(&[4, start as u64, c.count() as u64, chunk.len() as u64]);
        for (i, record) in chunk.iter().enumerate() {
            words[4 + i * 8..12 + i * 8].copy_from_slice(record)
        }
        let (r, n) = rpc_words(feedback, 4 + start as u64, &words[..4 + chunk.len() * 8]);
        assert_eq!(n, 8);
        assert_eq!(read64(&r, 0), start as u64);
    }
    native::exit(0)
}
