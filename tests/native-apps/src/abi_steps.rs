use native_userspace::{CounterRequest, Error, counter_value, native, read16, read32, read64};
// A sequence of public ABI operations and test inputs, not an implementation
// of the service, supervisor or ordinary application.
pub fn exercise(receiver: u64, feedback: u64) {
    if native::rpc(receiver, 90, &CounterRequest::Get.encode()) != Err(Error::Native(9))
        || native::lifecycle(1, 0, 0, 0)[0] != 11
    {
        native::exit(60);
    }
    let mut hello = [0u8; 32];
    hello[..8].copy_from_slice(&1u64.to_le_bytes());
    let (binding, n) = native::rpc(feedback, 1, &hello).unwrap_or_else(|_| native::exit(61));
    if n != 32 || read64(&binding, 0) == 0 {
        native::exit(62);
    }
    let sender = read64(&binding, 0);
    let (now, hz) = native::clock();
    let deadline = now.checked_add(hz).unwrap_or_else(|| native::exit(63));
    let (_, receipt) = native::call(1, sender, 10, deadline, &CounterRequest::Add(5).encode())
        .unwrap_or_else(|_| native::exit(64));
    native::call(5, receipt, 0, 0, &[]).unwrap_or_else(|_| native::exit(65));
    let (reply, _) = native::call(6, receipt, 0, 0, &[]).unwrap_or_else(|_| native::exit(66));
    if read16(&reply, 2) != 0 || read32(&reply, 16) != 16 || counter_value(&reply[24..40]) != Ok(5)
    {
        native::exit(67);
    }
    if !matches!(native::call(5, receipt, 0, 0, &[]), Err(Error::Native(2))) {
        native::exit(68);
    }
    for (id, request, expected) in [
        (11, CounterRequest::Add(7), 12),
        (12, CounterRequest::Get, 12),
    ] {
        let (reply, n) =
            native::rpc(sender, id, &request.encode()).unwrap_or_else(|_| native::exit(69));
        if counter_value(&reply[..n]) != Ok(expected) {
            native::exit(70);
        }
    }
    let mut done = [0u8; 32];
    for (i, word) in [2u64, 3, 12, 0].into_iter().enumerate() {
        done[i * 8..i * 8 + 8].copy_from_slice(&word.to_le_bytes());
    }
    native::rpc(feedback, 2, &done).unwrap_or_else(|_| native::exit(71));
}
