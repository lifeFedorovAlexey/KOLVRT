#![no_std]
#![no_main]
use native_userspace::{CounterRequest, Error, counter_value, native, read64};
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, feedback: u64, argument: u64) -> ! {
    // External ABI actor: these are invalid inputs to actual production methods.
    assert_eq!(native::lifecycle(1, 0, 0, 0)[0], 11);
    assert_eq!(
        native::rpc(receiver, 90, &CounterRequest::Get.encode()),
        Err(Error::Native(9))
    );
    assert!(matches!(
        native::call(4, receiver, 9999, 0, &[]),
        Err(Error::Native(2))
    ));
    if argument == 1 {
        assert!(native::clock().1 > 0);
        // SAFETY: only this isolated external EL0 test actor faults.
        unsafe {
            core::arch::asm!(".inst 0", options(noreturn));
        }
    }
    let (reply, n) = native::rpc(feedback, 1, &1u64.to_le_bytes()).unwrap();
    assert_eq!(n, 8);
    let old = read64(&reply, 0);
    let (reply, n) = native::rpc(old, 10, &CounterRequest::Get.encode()).unwrap();
    let first = counter_value(&reply[..n]).unwrap();
    assert_eq!(first, 12);
    let (reply, n) = native::rpc(feedback, 2, &first.to_le_bytes()).unwrap();
    assert_eq!(n, 8);
    let fresh = read64(&reply, 0);
    assert_ne!(fresh, old);
    assert_eq!(
        native::rpc(old, 11, &CounterRequest::Get.encode()),
        Err(Error::Native(2))
    );
    for (id, request, expected) in [
        (20, CounterRequest::Get, 0),
        (21, CounterRequest::Add(5), 5),
        (22, CounterRequest::Add(7), 12),
        (23, CounterRequest::Get, 12),
    ] {
        let (reply, n) = native::rpc(fresh, id, &request.encode()).unwrap();
        assert_eq!(counter_value(&reply[..n]), Ok(expected));
    }
    native::rpc(feedback, 3, &12u64.to_le_bytes()).unwrap();
    native::exit(0)
}
