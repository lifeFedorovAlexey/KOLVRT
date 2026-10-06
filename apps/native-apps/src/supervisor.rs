#![no_std]
#![no_main]
use native_userspace::{
    CounterRequest, Error, REPORT_MAGIC, counter_value, native, read32, read64,
};
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
#[unsafe(no_mangle)]
pub extern "C" fn native_main(_receiver: u64, _feedback: u64, argument: u64) -> ! {
    if argument == 1 {
        run_runtime();
    }
    // ELF images/placement/quotas are immutable kernel grants. Dependency and
    // readiness policy is EL0, using a completed response from the bound receiver.
    let original = life(1, 0, 0, if argument == 3 { 6 } else { 5 }, 10);
    probe(original[2], 1, 0, 20);
    let client = life(1, 1, 0, 0, 30);
    let binding = life(6, 1, client[1], original[1], 31);
    let (hello, hello_words) = receive(client[3], 50);
    if hello_words != [1, 0, 0, 0] {
        fail(54);
    }
    if argument == 2 || argument == 3 {
        let aux = life(1, 2, 0, 1, 120);
        if native::lifecycle(6, 2, aux[1], original[1])[0] != 11 {
            fail(121);
        }
        if argument == 2 {
            if completion(2, aux[1], 122)[1] != 2 {
                fail(123);
            }
        } else {
            let (aux_token, aux_words) = receive(aux[3], 124);
            if aux_words != [4, 9, 11, 0] {
                fail(127);
            }
            reply(aux[3], aux_token, [0, 0, 0, 0], 128);
            let ended = completion(2, aux[1], 129);
            if ended[1] != 1 || ended[2] != 0 {
                fail(130);
            }
        }
    }
    reply(client[3], hello, [binding[1], original[1], 0, 0], 55);
    let (done, done_words) = receive(client[3], 60);
    if done_words != [2, if argument == 3 { 4 } else { 3 }, 12, 0] {
        fail(64);
    }
    // Four requests have completed on one instance (ready + three client calls).
    // The fifth commits then faults, yielding an honest effect-unknown outcome.
    if !matches!(
        native::rpc(original[2], 2, &CounterRequest::Get.encode()),
        Err(Error::Outcome(5))
    ) {
        fail(70);
    }
    if completion(0, original[1], 71)[1] != 2 {
        fail(72);
    }
    let (ticks, frequency) = native::clock();
    let backoff = ticks
        .checked_add(frequency / 128)
        .unwrap_or_else(|| fail(73));
    while native::clock().0 < backoff {
        core::hint::spin_loop();
    }
    let fresh = life(3, 0, original[1], 0, 74);
    if fresh[1] == original[1] || fresh[4] == original[4] {
        fail(75);
    }
    if native::lifecycle(2, 0, original[1], 0)[0] != 2 {
        fail(76);
    }
    if !matches!(
        native::rpc(original[2], 3, &CounterRequest::Get.encode()),
        Err(Error::Native(2))
    ) {
        fail(77);
    }
    probe(fresh[2], 4, 0, 78);
    let new_binding = life(6, 1, client[1], fresh[1], 80);
    reply(client[3], done, [new_binding[1], fresh[1], 0, 0], 81);
    let (final_token, final_words) = receive(client[3], 90);
    if final_words != [3, if argument == 3 { 8 } else { 7 }, 12, 7] {
        fail(94);
    }
    reply(client[3], final_token, [0, 0, 0, 0], 95);
    let exited = completion(1, client[1], 96);
    if exited[1] != 1 || exited[2] != 0 {
        fail(97);
    }
    probe(fresh[2], 5, 12, 98);
    life(5, 0, 0, 0, 100);
    life(4, 0, fresh[1], 0, 110);
    if completion(0, fresh[1], 111)[1] != 3 {
        fail(112);
    }
    // Opaque userspace result; only the host runner knows this application schema.
    for word in [
        REPORT_MAGIC,
        1,
        0,
        1,
        fresh[1],
        final_words[1],
        12,
        1,
        1,
        1,
        0,
    ] {
        native::report(word);
    }
    native::exit(0)
}

fn run_runtime() -> ! {
    // Ordinary static runtime has no injected fault, restart fixture or shutdown.
    let service = life(1, 0, 0, 0, 140);
    probe(service[2], 1, 0, 141);
    let client = life(1, 1, 0, 0, 143);
    let binding = life(6, 1, client[1], service[1], 144);
    let (hello, words) = receive(client[3], 145);
    if words != [1, 0, 0, 0] {
        fail(148);
    }
    reply(client[3], hello, [binding[1], service[1], 0, 0], 149);
    let (done, words) = receive(client[3], 150);
    if words != [2, 3, 12, 0] {
        fail(153);
    }
    reply(client[3], done, [0, 0, 0, 0], 154);
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
    loop {
        probe(service[2], 4, 12, 162);
        let (now, hz) = native::clock();
        let until = now.checked_add(hz).unwrap_or_else(|| fail(164));
        while native::clock().0 < until {
            core::hint::spin_loop();
        }
    }
}
