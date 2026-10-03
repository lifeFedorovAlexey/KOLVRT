use kernel_core::wait::{Event, SharedEvent};
use std::sync::{Arc, Barrier};
#[test]
fn notifications_before_and_after_registration_are_retained() {
    let event = Event::new();
    assert!(event.signal());
    assert!(!event.signal());
    assert!(!event.register());
    assert!(event.register());
    assert!(!event.consume());
    assert!(event.signal());
    assert!(event.consume());
    assert!(!event.consume());
    event.signal();
    event.reset();
    assert!(event.register());
}
#[test]
fn publication_racing_registration_never_loses_wakeup() {
    for _ in 0..512 {
        let event = Arc::new(Event::new());
        let publisher = event.clone();
        let thread = std::thread::spawn(move || publisher.signal());
        let blocked = event.register();
        thread.join().unwrap();
        assert!(!blocked || event.consume());
        assert!(!event.consume());
    }
}
#[test]
fn delegated_event_references_survive_concurrent_signal_and_final_close() {
    for _ in 0..512 {
        let source = SharedEvent::try_new().unwrap();
        let delegated = source.try_clone().unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let first_barrier = barrier.clone();
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            source.signal()
        });
        let second_barrier = barrier.clone();
        let second = std::thread::spawn(move || {
            second_barrier.wait();
            delegated.signal()
        });
        barrier.wait();
        assert_ne!(first.join().unwrap(), second.join().unwrap());
        assert!(SharedEvent::try_new().is_some());
    }
}
#[test]
fn shared_event_reference_charge_is_bounded() {
    let event = SharedEvent::try_new().unwrap();
    let mut retained = Vec::new();
    for _ in 1..1024 {
        retained.push(event.try_clone().unwrap());
    }
    assert!(event.try_clone().is_none());
    retained.pop();
    assert!(event.try_clone().is_some());
}
