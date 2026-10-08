#![no_std]
#![no_main]
mod ipc_measure;
use ipc_measure::*;
use native_userspace::read64;
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, feedback: u64, argument: u64) -> ! {
    let c = Config::parse(argument);
    let mut sequence = 0u64;
    let mut digest = 0u64;
    let mut ready = true;
    loop {
        if c.case == 9 && ready {
            let (r, n) = rpc_words(feedback, 9000 + sequence, &[1, sequence]);
            assert_eq!(n, 8);
            assert_eq!(read64(&r, 0), 1);
            ready = false;
        }
        let (token, b, n) = receive(receiver);
        let id = read64(&b, 16);
        if n == 24 && read64(&b, 40) == MAGIC {
            reply(receiver, token, &[sequence, digest]);
            continue;
        }
        assert_eq!(n, c.payload());
        for i in 0..n {
            assert_eq!(b[40 + i], pattern(id, i))
        }
        sequence += 1;
        digest = digest.wrapping_add(hash(id, sequence));
        reply(receiver, token, &[1, sequence]);
        if c.case == 9 && sequence.is_multiple_of(2) {
            ready = true
        }
    }
}
