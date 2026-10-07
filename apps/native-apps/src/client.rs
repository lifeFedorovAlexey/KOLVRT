use native_apps::supervision::fresh_binding;
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
fn value(handle: u64, id: u64, request: CounterRequest) -> Result<u64, Error> {
    let (bytes, n) = native::rpc(handle, id, &request.encode())?;
    counter_value(&bytes[..n])
}
pub fn run(_receiver: u64, feedback: u64, _argument: u64) -> ! {
    let initial = control(feedback, 1, [1, 0, 0, 0]);
    let mut sender = initial[0];
    let mut instance = initial[1];
    if sender == 0 || instance == 0 {
        native::exit(44);
    }
    let mut completed = 0u64;
    let mut control_id = 2u64;
    loop {
        let mut failure = None;
        for (id, request, expected) in [
            (10, CounterRequest::Add(5), 5),
            (11, CounterRequest::Add(7), 12),
            (12, CounterRequest::Get, 12),
        ] {
            match value(sender, id, request) {
                Ok(actual) if actual == expected => {
                    completed = completed.checked_add(1).unwrap_or_else(|| native::exit(49));
                }
                Ok(_) => native::exit(45),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        let Some(error) = failure else {
            control(feedback, control_id, [2, completed, 12, 0]);
            native::exit(0);
        };
        let error_code = match error {
            Error::Native(code) => code,
            Error::Outcome(code) => 0x100 + u64::from(code),
            Error::Invalid => 1,
            Error::Overflow => 2,
        };
        let next = control(feedback, control_id, [3, instance, error_code, completed]);
        control_id = control_id
            .checked_add(1)
            .unwrap_or_else(|| native::exit(49));
        fresh_binding(instance, next).unwrap_or_else(|_| native::exit(46));
        if !matches!(
            native::rpc(sender, 13, &CounterRequest::Get.encode()),
            Err(Error::Native(2))
        ) {
            native::exit(47);
        }
        sender = next[0];
        instance = next[1];
        if value(sender, 20, CounterRequest::Get) != Ok(0) {
            native::exit(48);
        }
        // Repeat the business operation only in the verified fresh instance.
        // Never retry an effect-unknown ADD against the same instance.
    }
}
