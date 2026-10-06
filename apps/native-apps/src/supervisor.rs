#![no_std]
#![no_main]
use native_apps::supervision::recover_service;
use native_userspace::{CounterRequest, REPORT_MAGIC, counter_value, native, read32, read64};
native_userspace::entry!();

fn fail(code: u64) -> ! {
    for value in [REPORT_MAGIC, 1, code] {
        native::report(value);
    }
    native::exit(code)
}
fn checked(reply: [u64; 5], code: u64) -> [u64; 5] {
    if reply[0] != 0 {
        fail(code);
    }
    reply
}
fn life(op: u64, selector: u64, token: u64, argument: u64, code: u64) -> [u64; 5] {
    checked(native::lifecycle(op, selector, token, argument), code)
}
fn receive(receiver: u64, code: u64) -> (u64, [u64; 4]) {
    let (bytes, _) = native::call(2, receiver, 0, 0, &[]).unwrap_or_else(|_| fail(code));
    if read32(&bytes, 32) != 32 {
        fail(code + 1);
    }
    let token = read64(&bytes, 8);
    native::call(3, receiver, token, 0, &[]).unwrap_or_else(|_| fail(code + 2));
    (token, core::array::from_fn(|i| read64(&bytes, 40 + i * 8)))
}
fn reply(receiver: u64, token: u64, values: [u64; 4], code: u64) {
    let mut bytes = [0; 32];
    for (i, value) in values.iter().enumerate() {
        bytes[i * 8..i * 8 + 8].copy_from_slice(&value.to_le_bytes());
    }
    native::call(4, receiver, token, 0, &bytes).unwrap_or_else(|_| fail(code));
}
fn probe(sender: u64, id: u64, expected: u64, code: u64) {
    let (bytes, n) =
        native::rpc(sender, id, &CounterRequest::Get.encode()).unwrap_or_else(|_| fail(code));
    if counter_value(&bytes[..n]) != Ok(expected) {
        fail(code + 1);
    }
}
fn completion(selector: u64, token: u64, code: u64) -> [u64; 5] {
    loop {
        let result = life(2, selector, token, 0, code);
        if result[1] != 0 {
            return result;
        }
    }
}
fn recover(old: [u64; 5], code: u64) -> [u64; 5] {
    recover_service(old).unwrap_or_else(|_| fail(code))
}
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, _feedback: u64, _argument: u64) -> ! {
    run_runtime()
}
fn run_runtime() -> ! {
    // Actual lifecycle observations govern recovery; no injected failure path.
    let mut service = life(1, 0, 0, 0, 140);
    probe(service[2], 1, 0, 141);
    let client = life(1, 1, 0, 0, 143);
    let binding = life(6, 1, client[1], service[1], 144);
    let (hello, words) = receive(client[3], 145);
    if words != [1, 0, 0, 0] {
        fail(148);
    }
    reply(client[3], hello, [binding[1], service[1], 0, 0], 149);
    loop {
        let (message, words) = receive(client[3], 150);
        match words[0] {
            2 if words[1] >= 3 && words[2] == 12 && words[3] == 0 => {
                probe(service[2], 2, 12, 157);
                reply(client[3], message, [0, 0, 0, 0], 154);
                break;
            }
            3 if words[1] == service[1] => {
                service = recover(service, 170);
                let binding = life(6, 1, client[1], service[1], 180);
                reply(client[3], message, [binding[1], service[1], 0, 0], 181);
            }
            _ => fail(153),
        }
    }
    let exited = completion(1, client[1], 155);
    if exited[1] != 1 || exited[2] != 0 {
        fail(156);
    }
    probe(service[2], 2, 12, 157);
    life(5, 0, 0, 0, 159);
    for word in [REPORT_MAGIC, 2, 12, service[1], 0] {
        native::report(word);
    }
    probe(service[2], 3, 12, 160);
    native::report(1);
    let mut expected = 12;
    loop {
        match native::rpc(service[2], 4, &CounterRequest::Get.encode()) {
            Ok((bytes, n)) => {
                if counter_value(&bytes[..n]) != Ok(expected) {
                    fail(163);
                }
            }
            Err(_) => {
                service = recover(service, 190);
                expected = 0;
            }
        }
        let (now, hz) = native::clock();
        let until = now.checked_add(hz).unwrap_or_else(|| fail(164));
        while native::clock().0 < until {
            core::hint::spin_loop();
        }
    }
}
