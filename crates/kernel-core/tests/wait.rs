use kernel_core::wait::Event;
use std::sync::Arc;
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
