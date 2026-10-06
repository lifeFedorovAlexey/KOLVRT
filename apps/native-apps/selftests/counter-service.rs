#![no_std]
#![no_main]
use native_userspace::{
    Counter, CounterRequest, Error, HEADER_BYTES, counter_reply, native, read32, read64,
};
native_userspace::entry!();

#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, _feedback: u64, argument: u64) -> ! {
    let mut state = Counter::new();
    let mut completed = 0u64;
    loop {
        let (delivery, _) =
            native::call(2, receiver, 0, 0, &[]).unwrap_or_else(|_| native::exit(10));
        let token = read64(&delivery, 8);
        let length = read32(&delivery, 32) as usize;
        if length > 256 {
            native::exit(11);
        }
        let request = CounterRequest::decode(&delivery[HEADER_BYTES..HEADER_BYTES + length]);
        native::call(3, receiver, token, 0, &[]).unwrap_or_else(|_| native::exit(12));
        // A development launch argument injects a genuine process fault after commitment.
        // The normal service has no crash RPC or reset operation and runs indefinitely.
        if argument != 0 {
            completed = completed.checked_add(1).unwrap_or_else(|| native::exit(13));
            if completed == argument {
                native::fault();
            }
        }
        let value = request.and_then(|request| state.apply(request));
        #[cfg(feature = "reply-negative")]
        let value = if completed == 3 {
            value.map(|v| v ^ 1)
        } else {
            value
        };
        let payload = counter_reply(value.map_err(|error| match error {
            Error::Overflow => Error::Overflow,
            _ => Error::Invalid,
        }));
        native::call(4, receiver, token, 0, &payload).unwrap_or_else(|_| native::exit(14));
    }
}
