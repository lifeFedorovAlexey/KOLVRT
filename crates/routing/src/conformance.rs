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
        let mut retained = Consumer::from_profile(&profile, 0).unwrap();
        core::mem::forget(retained.begin().unwrap());
        check(
            "routing_forgotten_transaction",
            retained.switch(Route::Native) == Err(Error::Busy) && retained.begin().is_err(),
        );
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
    fn integrity_rejects_every_single_byte_mutation() {
        let original = super::Profile::native().encode();
        let expected = super::digest(&original);
        for index in 0..original.len() {
            let mut bytes = original;
            bytes[index] ^= 1;
            assert_eq!(
                super::Profile::decode(&bytes, expected),
                Err(super::Error::Integrity)
            );
        }
    }
    #[test]
    fn native_denial_is_not_replaced_by_fallback() {
        struct Denied;
        impl kernel_core::window::Backend for Denied {
            fn reduce(
                &mut self,
                _: kernel_core::window::Span,
            ) -> Result<kernel_core::window::Reduction, kernel_core::window::Error> {
                Err(kernel_core::window::Error::Bounds)
            }
        }
        for route in super::Route::ALL.into_iter().filter(|r| r.available()) {
            let profile = super::Profile::new(1, [route; super::CONSUMERS]).unwrap();
            let mut consumer = super::Consumer::from_profile(&profile, 0).unwrap();
            let inclusive = [0, 0, 0, 0];
            let counted = [0, 0, 0, 0, 0, 0, 0, 1];
            let input = match route {
                super::Route::Native => super::Input::Native(super::Span { start: 0, end: 1 }),
                super::Route::Inclusive => super::Input::Encoded(&inclusive),
                _ => super::Input::Encoded(&counted),
            };
            assert_eq!(
                consumer.call_with(&mut Denied, input),
                Err(super::Error::Bounds)
            );
        }
    }
    #[test]
    fn same_kernel_contracts() {
        super::run(|name, passed| assert!(passed, "{name}"));
    }

    #[test]
    fn public_metadata_and_profile_accessors_cover_every_variant() {
        for route in super::Route::ALL {
            assert!(super::Route::parse(route as u8).is_ok());
            assert!(!route.name().is_empty());
            assert!(!route.reason().is_empty());
            assert_ne!(route.identity(), [0; 32]);
        }

        let routes = super::Route::ALL.map(|route| {
            if route.available() {
                route
            } else {
                super::Route::Native
            }
        });
        let profile = super::Profile::new(7, routes).unwrap();
        assert_eq!(profile.generation(), 7);
        assert_eq!(profile.routes(), &routes);

        for status in [
            super::Status::Unknown,
            super::Status::Legacy,
            super::Status::Compat,
            super::Status::Mixed,
            super::Status::MostlyNative,
            super::Status::Native,
        ] {
            assert!(!status.name().is_empty());
            assert!(!status.color().is_empty());
        }
    }

    #[cfg(feature = "dev")]
    #[test]
    fn transaction_charges_ticks_to_the_bound_route() {
        let mut consumer = super::Consumer::from_profile(&super::Profile::native(), 0).unwrap();
        {
            let mut transaction = consumer.begin().unwrap();
            transaction.charge_ticks(17);
        }
        assert_eq!(consumer.counters(super::Route::Native).ticks, 17);
    }

    #[test]
    fn compat_routes_reject_native_input() {
        let data = [1, 2, 3];
        for route in super::Route::ALL
            .into_iter()
            .filter(|route| *route != super::Route::Native && route.available())
        {
            let profile = super::Profile::new(1, [route; super::CONSUMERS]).unwrap();
            let mut consumer = super::Consumer::from_profile(&profile, 0).unwrap();
            assert_eq!(
                consumer.call(
                    &data,
                    super::Input::Native(kernel_core::window::Span { start: 0, end: 1 }),
                ),
                Err(super::Error::Encoding),
            );
        }
    }

    #[cfg(all(feature = "dev", not(feature = "compat-v1")))]
    #[test]
    fn unavailable_route_cannot_be_added_to_a_profile_or_switched_in() {
        assert!(!super::Route::Inclusive.available());
        assert_eq!(
            super::Profile::new(1, [super::Route::Inclusive; super::CONSUMERS]),
            Err(super::Error::Unsupported),
        );
        let mut consumer = super::Consumer::from_profile(&super::Profile::native(), 0).unwrap();
        assert_eq!(consumer.switch(super::Route::Inclusive), Err(super::Error::Unsupported));
        assert_eq!(consumer.route(), super::Route::Native);
        assert_eq!(consumer.generation(), 1);
    }
}
