use native_userspace::{CounterRequest, Error, counter_value, native, read64};

fn words(values: [u64; 4]) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (i, value) in values.iter().enumerate() {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&value.to_le_bytes());
    }
    bytes
}
fn control(feedback: u64, id: u64, values: [u64; 4]) -> [u64; 4] {
    let (bytes, length) =
        native::rpc(feedback, id, &words(values)).unwrap_or_else(|_| native::exit(40));
    if length != 32 {
        native::exit(41);
    }
    core::array::from_fn(|i| read64(&bytes, i * 8))
}
fn value(handle: u64, id: u64, request: CounterRequest) -> u64 {
    let (bytes, length) =
        native::rpc(handle, id, &request.encode()).unwrap_or_else(|_| native::exit(42));
    counter_value(&bytes[..length]).unwrap_or_else(|_| native::exit(43))
}
pub fn run(receiver: u64, feedback: u64, argument: u64, suite: u64) -> ! {
    if argument == 1 {
        if suite == 2 {
            native::fault();
        }
        if suite == 3 {
            if native::rpc(receiver, 90, &CounterRequest::Get.encode()) != Err(Error::Native(9))
                || native::lifecycle(1, 0, 0, 0)[0] != 11
            {
                native::exit(60);
            }
            control(feedback, 91, [4, 9, 11, 0]);
            native::exit(0);
        }
    }
    if suite == 5 && native::clock().1 == 0 {
        native::exit(61);
    }
    let initial = control(feedback, 1, [1, 0, 0, 0]);
    let old = initial[0];
    if old == 0 || initial[1] == 0 {
        native::exit(44);
    }
    let mut extra = 0u64;
    if suite == 3 {
        if native::rpc(receiver, 92, &CounterRequest::Get.encode()) != Err(Error::Native(9))
            || native::lifecycle(1, 0, 0, 0)[0] != 11
        {
            native::exit(62);
        }
        let (ticks, hz) = native::clock();
        let (_, receipt) = native::call(
            1,
            old,
            93,
            ticks.checked_add(hz).unwrap_or_else(|| native::exit(63)),
            &CounterRequest::Get.encode(),
        )
        .unwrap_or_else(|_| native::exit(64));
        native::call(5, receipt, 0, 0, &[]).unwrap_or_else(|_| native::exit(65));
        let (collected, _) =
            native::call(6, receipt, 0, 0, &[]).unwrap_or_else(|_| native::exit(66));
        if native_userspace::read16(&collected, 2) != 0
            || native_userspace::read32(&collected, 16) != 16
            || counter_value(&collected[24..40]) != Ok(0)
        {
            native::exit(68);
        }
        if !matches!(native::call(5, receipt, 0, 0, &[]), Err(Error::Native(2))) {
            native::exit(67);
        }
        extra = 1;
    }
    if value(old, 10, CounterRequest::Add(5)) != 5
        || value(old, 11, CounterRequest::Add(7)) != 12
        || value(old, 12, CounterRequest::Get) != 12
    {
        native::exit(45);
    }
    if suite == 1 {
        control(feedback, 2, [2, 3, 12, 0]);
        native::exit(0);
    }
    // The client remains alive while the supervisor crashes and replaces the service.
    let fresh = control(feedback, 2, [2, 3 + extra, 12, 0]);
    if fresh[0] == 0 || fresh[1] == initial[1] || fresh[2] != 0 {
        native::exit(46);
    }
    match native::rpc(old, 13, &CounterRequest::Get.encode()) {
        Err(Error::Native(2)) => {}
        _ => native::exit(47),
    }
    if value(fresh[0], 20, CounterRequest::Get) != 0
        || value(fresh[0], 21, CounterRequest::Add(5)) != 5
        || value(fresh[0], 22, CounterRequest::Add(7)) != 12
        || value(fresh[0], 23, CounterRequest::Get) != 12
    {
        native::exit(48);
    }
    control(feedback, 3, [3, 7 + extra, 12, 7]);
    native::exit(0)
}
