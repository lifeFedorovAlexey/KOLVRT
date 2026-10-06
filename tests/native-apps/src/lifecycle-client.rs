#![no_std]
#![no_main]
use native_userspace::{
    CounterRequest, Error, REPORT_MAGIC, counter_value, native, read32, read64,
};
native_userspace::entry!();
// Public lifecycle ABI test steps. No readiness/recovery policy implementation,
// production supervisor loop, service state or duplicated application methods.
fn note(step: u64, a: u64, b: u64, c: u64) {
    for word in [step, a, b, c] {
        native::report(word);
    }
}
fn terminal(selector: u64, token: u64) -> [u64; 5] {
    loop {
        let result = native::lifecycle(2, selector, token, 0);
        assert_eq!(result[0], 0);
        if result[1] != 0 {
            return result;
        }
    }
}
fn exchange(sender: u64, id: u64, request: CounterRequest) -> u64 {
    let (reply, n) = native::rpc(sender, id, &request.encode()).unwrap();
    counter_value(&reply[..n]).unwrap()
}
fn receive(receiver: u64) -> (u64, u64) {
    let (bytes, _) = native::call(2, receiver, 0, 0, &[]).unwrap();
    assert_eq!(read32(&bytes, 32), 8);
    let token = read64(&bytes, 8);
    native::call(3, receiver, token, 0, &[]).unwrap();
    (token, read64(&bytes, 40))
}
fn answer(receiver: u64, token: u64, value: u64) {
    native::call(4, receiver, token, 0, &value.to_le_bytes()).unwrap();
}
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, _feedback: u64, _argument: u64) -> ! {
    for word in [REPORT_MAGIC, 3] {
        native::report(word);
    }
    // 1. Launch the actual production counter ELF. Replacement while live is denied.
    let old = native::lifecycle(1, 0, 0, 0);
    assert_eq!(old[0], 0);
    note(10, old[1], old[4], 0);
    let busy = native::lifecycle(3, 0, old[1], 0);
    assert_eq!(busy[0], 13);
    note(11, busy[0], 0, 0);
    assert_eq!(exchange(old[2], 1, CounterRequest::Get), 0);
    let a = exchange(old[2], 10, CounterRequest::Add(5));
    let b = exchange(old[2], 11, CounterRequest::Add(7));
    let c = exchange(old[2], 12, CounterRequest::Get);
    assert_eq!((a, b, c), (5, 12, 12));
    note(12, a, b, c);
    // 2. An external ABI client blocks on ordinary IPC while its binding is tested.
    let client = native::lifecycle(1, 1, 0, 0);
    assert_eq!(client[0], 0);
    let (message, value) = receive(client[3]);
    assert_eq!(value, 1);
    let wrong = native::lifecycle(6, 1, client[1], 0);
    assert_eq!(wrong[0], 2);
    let binding = native::lifecycle(6, 1, client[1], old[1]);
    assert_eq!(binding[0], 0);
    note(13, wrong[0], binding[0], binding[1]);
    answer(client[3], message, binding[1]);
    let (message, value) = receive(client[3]);
    assert_eq!(value, 12);
    note(14, value, 0, 0);
    // 3. Stop and replace through the real owner interface. No injected crash.
    assert_eq!(native::lifecycle(4, 0, old[1], 0)[0], 0);
    let stopped = terminal(0, old[1]);
    assert_eq!(stopped[1], 3);
    // INTEGRATION: call the same production recovery method as native-supervisor.
    let fresh = native_apps::supervision::recover_service(old).unwrap();
    assert_eq!(fresh[0], 0);
    assert_ne!(fresh[1], old[1]);
    assert_ne!(fresh[4], old[4]);
    note(15, stopped[1], fresh[1], fresh[4]);
    let stale = native::lifecycle(2, 0, old[1], 0);
    assert_eq!(stale[0], 2);
    let stale_send = match native::rpc(old[2], 13, &CounterRequest::Get.encode()) {
        Err(Error::Native(code)) => code,
        _ => native::exit(160),
    };
    assert_eq!(stale_send, 2);
    let initial = exchange(fresh[2], 20, CounterRequest::Get);
    assert_eq!(initial, 0);
    note(16, stale[0], stale_send, initial);
    let binding = native::lifecycle(6, 1, client[1], fresh[1]);
    assert_eq!(binding[0], 0);
    note(17, binding[0], binding[1], 0);
    answer(client[3], message, binding[1]);
    let (message, value) = receive(client[3]);
    assert_eq!(value, 12);
    answer(client[3], message, 0);
    let ended = terminal(1, client[1]);
    assert_eq!((ended[1], ended[2]), (1, 0));
    note(18, ended[1], ended[2], 0);
    // 4. Fault only an external actor; the production service continues afterwards.
    let peer = native::lifecycle(1, 2, 0, 1);
    assert_eq!(peer[0], 0);
    let denied = native::lifecycle(6, 2, peer[1], fresh[1]);
    assert_eq!(denied[0], 11);
    let fault = terminal(2, peer[1]);
    assert_eq!(fault[1], 2);
    note(19, denied[0], fault[1], fault[2]);
    let healthy = exchange(fresh[2], 21, CounterRequest::Get);
    assert_eq!(healthy, 12);
    note(20, healthy, 0, 0);
    // 5. End the owner session normally; the kernel must reclaim actual resources.
    assert_eq!(native::lifecycle(4, 0, fresh[1], 0)[0], 0);
    let stopped = terminal(0, fresh[1]);
    assert_eq!(stopped[1], 3);
    note(21, stopped[1], 0, 0);
    native::exit(0)
}
