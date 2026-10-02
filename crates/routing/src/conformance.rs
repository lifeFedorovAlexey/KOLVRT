use super::*;
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(&bytes[..PROFILE_BYTES - DIGEST_BYTES]).into()
}
fn seal(bytes: &mut [u8; PROFILE_BYTES]) -> [u8; 32] {
    let hash = digest(bytes);
    bytes[PROFILE_BYTES - DIGEST_BYTES..].copy_from_slice(&hash);
    hash
}
pub fn run(mut check: impl FnMut(&str, bool)) {
    let data = [10, 20, 30, 40];
    let profile = Profile::native();
    let mut native = Consumer::from_profile(&profile, 0).unwrap();
    check(
        "routing_native",
        native
            .call(&data, Input::Native(Span { start: 1, end: 3 }))
            .unwrap()
            .0
            == Reduction { sum: 50, words: 2 },
    );
    check(
        "routing_native_empty",
        native
            .call(&data, Input::Native(Span { start: 0, end: 0 }))
            .unwrap()
            .0
            .sum
            == 0,
    );
    check(
        "routing_bounds",
        native.call(&data, Input::Native(Span { start: 3, end: 2 })) == Err(Error::Bounds)
            && native.call(&data, Input::Native(Span { start: 0, end: 5 })) == Err(Error::Bounds),
    );
    check(
        "routing_invalid",
        Route::parse(9) == Err(Error::Unsupported)
            && native.call(&data, Input::Encoded(&[0; 4])) == Err(Error::Encoding)
            && Consumer::from_profile(&profile, CONSUMERS).is_err(),
    );
    let bytes = profile.encode();
    let expected = digest(&bytes);
    check(
        "routing_profile",
        Profile::decode(&bytes, expected) == Ok(profile),
    );
    let mut changed = bytes;
    changed[SCHEMA_OFFSET + 1] = 1;
    let hash = seal(&mut changed);
    check(
        "routing_profile_version",
        Profile::decode(&changed, hash) == Err(Error::ProfileVersion),
    );
    changed = bytes;
    changed[NATIVE_VERSION_OFFSET + 1] = 1;
    let hash = seal(&mut changed);
    check(
        "routing_native_version",
        Profile::decode(&changed, hash) == Err(Error::ProfileVersion),
    );
    changed = bytes;
    changed[HEADER + ENTRY_IDENTITY_OFFSET] ^= 1;
    let hash = seal(&mut changed);
    check(
        "routing_profile_identity",
        Profile::decode(&changed, hash) == Err(Error::Identity),
    );
    changed = bytes;
    changed[HEADER] = 1;
    let hash = seal(&mut changed);
    check(
        "routing_profile_consumer",
        Profile::decode(&changed, hash) == Err(Error::Consumer),
    );
    changed = bytes;
    changed[GENERATION_OFFSET] ^= 1;
    check(
        "routing_profile_integrity",
        Profile::decode(&changed, expected) == Err(Error::Integrity)
            && Profile::decode(&bytes, [0; 32]) == Err(Error::Integrity)
            && Profile::decode(&bytes[..7], expected) == Err(Error::Encoding),
    );
    check(
        "routing_profile_generation",
        Profile::new(0, [Route::Native; CONSUMERS]) == Err(Error::Generation),
    );
    #[cfg(not(feature = "dev"))]
    check(
        "routing_no_instrumentation",
        core::mem::size_of::<Consumer>() <= 16,
    );
    #[cfg(feature = "dev")]
    {
        let generation = native.generation();
        let mut tx = native.begin().unwrap();
        check(
            "routing_unsafe_switch",
            tx.try_switch(Route::Native) == Err(Error::Busy),
        );
        drop(tx);
        native.switch(Route::Native).unwrap();
        check("routing_safe_switch", native.generation() == generation + 1);
        let mut full = Consumer {
            generation: u32::MAX,
            route: Route::Native,
            busy: false,
            counters: [Counters::default(); ROUTE_COUNT],
        };
        check(
            "routing_generation_exhaustion",
            full.switch(Route::Native) == Err(Error::Generation) && full.generation() == u32::MAX,
        );
        let c = native.counters(Route::Native);
        check(
            "routing_accounting",
            c.native_calls == 5
                && c.backend_calls == 4
                && c.errors == 3
                && c.compat_calls == 0
                && c.translations == 0
                && c.fallbacks == 0,
        );
        let c = Counters {
            native_calls: 31,
            compat_calls: 1,
            ..Counters::default()
        };
        check(
            "routing_status",
            classify(true, false, Counters::default(), None) == Status::Unknown
                && classify(
                    true,
                    false,
                    Counters {
                        native_calls: 1,
                        ..Counters::default()
                    },
                    None,
                ) == Status::Native
                && classify(false, true, c, None) == Status::Legacy
                && classify(
                    true,
                    true,
                    Counters {
                        compat_calls: 1,
                        ..Counters::default()
                    },
                    None,
                ) == Status::Compat
                && classify(true, true, c, None) == Status::Mixed
                && classify(true, true, c, Some(32)) == Status::MostlyNative,
        );
    }
    for route in Route::ALL {
        if !route.available() {
            check(
                "routing_missing_module",
                Profile::new(1, [route; CONSUMERS]) == Err(Error::Unsupported),
            );
            break;
        }
    }
    #[cfg(all(feature = "compat-v1", feature = "compat-v2"))]
    {
        let p = Profile::new(
            2,
            [
                Route::Native,
                Route::Inclusive,
                Route::Counted,
                Route::Native,
            ],
        )
        .unwrap();
        let mut a = Consumer::from_profile(&p, 0).unwrap();
        let mut b = Consumer::from_profile(&p, 1).unwrap();
        let mut c = Consumer::from_profile(&p, 2).unwrap();
        let first = [1, 0, 2, 0];
        let counted = [0, 0, 0, 1, 0, 0, 0, 2];
        for _ in 0..3 {
            check_once(&mut a, &mut b, &mut c, &data, &first, &counted);
        }
        check(
            "routing_simultaneous_versions",
            b.route() == Route::Inclusive
                && c.route() == Route::Counted
                && a.route() == Route::Native,
        );
        check(
            "routing_encoding_rejection",
            b.call(&data, Input::Encoded(&counted)) == Err(Error::Encoding)
                && c.call(&data, Input::Encoded(&first)) == Err(Error::Encoding),
        );
        check(
            "routing_translation_bounds",
            c.call(&data, Input::Encoded(&[255; 8])) == Err(Error::Bounds)
                && b.call(&data, Input::Encoded(&[3, 0, 2, 0])) == Err(Error::Bounds),
        );
        check(
            "routing_empty_versions",
            b.call(&data, Input::Encoded(&[255; 4])).unwrap().0.sum == 0
                && c.call(&data, Input::Encoded(&[0; 8])).unwrap().0.sum == 0,
        );
        #[cfg(feature = "dev")]
        {
            let bc = b.counters(Route::Inclusive);
            let cc = c.counters(Route::Counted);
            check(
                "routing_translation_accounting",
                bc.compat_calls == 6
                    && bc.translations == 5
                    && bc.copies == 5
                    && bc.copied_bytes == 20
                    && bc.conversions == 13
                    && bc.backend_calls == 4
                    && bc.errors == 2
                    && cc.compat_calls == 6
                    && cc.translations == 5
                    && cc.copied_bytes == 40
                    && cc.conversions == 14
                    && cc.backend_calls == 4
                    && cc.errors == 2,
            );
            b.switch(Route::Native).unwrap();
            check(
                "routing_isolation",
                a.route() == Route::Native
                    && c.route() == Route::Counted
                    && b.route() == Route::Native
                    && c.generation() == 2,
            );
        }
    }
    #[cfg(feature = "bug-compat")]
    {
        let p = Profile::new(3, [Route::EmptyFirst; CONSUMERS]).unwrap();
        let mut old = Consumer::from_profile(&p, 0).unwrap();
        check(
            "routing_bug_compat",
            old.call(&data, Input::Encoded(&[0; 8])).unwrap().0 == Reduction { sum: 10, words: 1 }
                && native
                    .call(&data, Input::Native(Span { start: 0, end: 0 }))
                    .unwrap()
                    .0
                    .sum
                    == 0,
        );
        check(
            "routing_bug_bounds",
            old.call(&[], Input::Encoded(&[0; 8])) == Err(Error::Bounds),
        );
    }
}
#[cfg(all(feature = "compat-v1", feature = "compat-v2"))]
fn check_once(
    a: &mut Consumer,
    b: &mut Consumer,
    c: &mut Consumer,
    data: &[u32],
    first: &[u8],
    counted: &[u8],
) {
    let n = a
        .call(data, Input::Native(Span { start: 1, end: 3 }))
        .unwrap()
        .0;
    assert_eq!(b.call(data, Input::Encoded(first)).unwrap().0, n);
    assert_eq!(c.call(data, Input::Encoded(counted)).unwrap().0, n);
}
#[cfg(test)]
mod host {
    #[test]
    fn same_kernel_contracts() {
        super::run(|name, passed| assert!(passed, "{name}"));
    }
}
