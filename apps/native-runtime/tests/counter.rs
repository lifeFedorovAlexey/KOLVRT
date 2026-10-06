use native_userspace::*;
#[test]
fn repeated_state_and_restart_initial_value() {
    let mut state = Counter::new();
    assert_eq!(state.apply(CounterRequest::Add(5)), Ok(5));
    assert_eq!(state.apply(CounterRequest::Add(7)), Ok(12));
    assert_eq!(state.apply(CounterRequest::Get), Ok(12));
    assert_eq!(Counter::new().value(), 0);
}
#[test]
fn overflow_does_not_mutate_state() {
    let mut state = Counter::new();
    state.apply(CounterRequest::Add(u64::MAX)).unwrap();
    assert_eq!(state.apply(CounterRequest::Add(1)), Err(Error::Overflow));
    assert_eq!(state.value(), u64::MAX);
}
#[test]
fn parser_rejects_reserved_fields_versions_lengths_and_get_delta() {
    for request in [CounterRequest::Get, CounterRequest::Add(u64::MAX)] {
        assert_eq!(CounterRequest::decode(&request.encode()), Ok(request));
    }
    let good = CounterRequest::Get.encode();
    for n in 0..16 {
        assert_eq!(CounterRequest::decode(&good[..n]), Err(Error::Invalid));
    }
    for at in [0, 2, 4, 8] {
        let mut bad = good;
        bad[at] = 99;
        assert_eq!(CounterRequest::decode(&bad), Err(Error::Invalid));
    }
}
#[test]
fn native_frames_are_explicit_initialized_little_endian() {
    let payload = CounterRequest::Add(7).encode();
    let (bytes, n) = frame(1, 9, 11, 13, &payload);
    assert_eq!(n, 56);
    assert_eq!(read16(&bytes, 0), 1);
    assert_eq!(read64(&bytes, 8), 9);
    assert_eq!(read64(&bytes, 16), 11);
    assert_eq!(read64(&bytes, 24), 13);
    assert_eq!(read32(&bytes, 32), 16);
    assert!(bytes[n..].iter().all(|b| *b == 0));
    assert_eq!(counter_value(&counter_reply(Ok(12))), Ok(12));
}

#[test]
fn reply_parser_rejects_malformed_version_reserved_status_and_length() {
    let good = counter_reply(Ok(12));
    assert_eq!(counter_value(&good), Ok(12));
    for length in 0..16 {
        assert_eq!(counter_value(&good[..length]), Err(Error::Invalid));
    }
    for offset in [0, 2, 4] {
        let mut bad = good;
        bad[offset] = 99;
        assert_eq!(counter_value(&bad), Err(Error::Invalid));
    }
    assert_eq!(
        counter_value(&counter_reply(Err(Error::Overflow))),
        Err(Error::Overflow)
    );
    assert_eq!(
        counter_value(&counter_reply(Err(Error::Invalid))),
        Err(Error::Invalid)
    );
}
