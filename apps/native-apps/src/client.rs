use native_userspace::{CounterRequest, counter_value, native, read64};

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
pub fn run(_receiver: u64, feedback: u64, _argument: u64) -> ! {
    let initial = control(feedback, 1, [1, 0, 0, 0]);
    let sender = initial[0];
    if sender == 0 || initial[1] == 0 {
        native::exit(44);
    }
    if value(sender, 10, CounterRequest::Add(5)) != 5
        || value(sender, 11, CounterRequest::Add(7)) != 12
        || value(sender, 12, CounterRequest::Get) != 12
    {
        native::exit(45);
    }
    control(feedback, 2, [2, 3, 12, 0]);
    native::exit(0)
}
