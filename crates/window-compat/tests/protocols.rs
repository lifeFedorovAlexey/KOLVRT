#[cfg(feature = "bug")]
use kernel_core::window::Error as NativeError;
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
use kernel_core::window::{Local, Reduction};
#[cfg(any(feature = "v1", feature = "v2", feature = "bug"))]
use window_compat::{Error, Work};

#[cfg(feature = "v1")]
#[test]
fn inclusive_v1_covers_snapshot_sentinel_bounds_and_native_rejection() {
    let data = [5, 7, 11, 13];
    let (result, work) = window_compat::v1(&data, &[1, 0, 2, 0]);
    assert_eq!(result, Ok(Reduction { sum: 18, words: 2 }));
    assert_eq!(
        work,
        Work {
            translations: 1,
            copied_bytes: 4,
            conversions: 3,
            backend_calls: 1,
        }
    );

    assert_eq!(
        window_compat::v1(&data, &[0xff; 4]),
        (
            Ok(Reduction { sum: 0, words: 0 }),
            Work {
                translations: 1,
                copied_bytes: 4,
                conversions: 2,
                backend_calls: 1,
            }
        )
    );
    assert_eq!(window_compat::v1(&data, &[0; 3]).0, Err(Error::Encoding));
    assert_eq!(
        window_compat::v1(&data, &[2, 0, 1, 0]).0,
        Err(Error::Bounds)
    );
    assert_eq!(
        window_compat::v1(&data, &[0, 0, 9, 0]).0,
        Err(Error::Bounds)
    );

    let mut backend = Local(&data);
    assert_eq!(
        window_compat::v1_with(&mut backend, &[0, 0, 0, 0]).0,
        Ok(Reduction { sum: 5, words: 1 })
    );
}

#[cfg(feature = "v2")]
#[test]
fn counted_v2_covers_zero_count_encoding_overflow_and_native_rejection() {
    let data = [3, 5, 7, 11];
    assert_eq!(
        window_compat::v2(&data, &[0, 0, 0, 1, 0, 0, 0, 2]),
        (
            Ok(Reduction { sum: 12, words: 2 }),
            Work {
                translations: 1,
                copied_bytes: 8,
                conversions: 3,
                backend_calls: 1,
            }
        )
    );
    assert_eq!(
        window_compat::v2(&data, &[0, 0, 0, 2, 0, 0, 0, 0]).0,
        Ok(Reduction { sum: 0, words: 0 })
    );
    assert_eq!(window_compat::v2(&data, &[0; 7]).0, Err(Error::Encoding));
    assert_eq!(
        window_compat::v2(&data, &[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 1]).0,
        Err(Error::Bounds)
    );
    assert_eq!(
        window_compat::v2(&data, &[0, 0, 0, 3, 0, 0, 0, 2]).0,
        Err(Error::Bounds)
    );

    let mut backend = Local(&data);
    assert_eq!(
        window_compat::v2_with(&mut backend, &[0, 0, 0, 0, 0, 0, 0, 1]).0,
        Ok(Reduction { sum: 3, words: 1 })
    );
}

#[cfg(feature = "bug")]
#[test]
fn empty_first_bug_route_only_changes_zero_count_behavior() {
    let data = [17, 19];
    assert_eq!(
        window_compat::empty_first(&data, &[0, 0, 0, 0, 0, 0, 0, 0]),
        (
            Ok(Reduction { sum: 17, words: 1 }),
            Work {
                translations: 1,
                copied_bytes: 8,
                conversions: 4,
                backend_calls: 1,
            }
        )
    );
    assert_eq!(
        window_compat::empty_first(&data, &[0, 0, 0, 0, 0, 0, 0, 1]).0,
        Ok(Reduction { sum: 17, words: 1 })
    );
    assert_eq!(
        window_compat::empty_first(&data, &[0, 0, 0, 1, 0, 0, 0, 0]).0,
        Ok(Reduction { sum: 19, words: 1 })
    );
    assert_eq!(
        window_compat::empty_first(&data, &[0, 0, 0, 0, 0, 0, 0]).0,
        Err(Error::Encoding)
    );
    assert_eq!(
        window_compat::empty_first(&data, &[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]).0,
        Err(Error::Bounds)
    );

    let mut backend = Local(&data);
    assert_eq!(
        window_compat::empty_first_with(&mut backend, &[0, 0, 0, 0, 0, 0, 0, 1]).0,
        Ok(Reduction { sum: 17, words: 1 })
    );
    let mut rejecting = Reject;
    assert_eq!(
        window_compat::empty_first_with(&mut rejecting, &[0; 8]),
        (
            Err(Error::Bounds),
            Work {
                translations: 1,
                copied_bytes: 8,
                conversions: 4,
                backend_calls: 1,
            }
        )
    );
}

#[cfg(feature = "bug")]
struct Reject;

#[cfg(feature = "bug")]
impl kernel_core::window::Backend for Reject {
    fn reduce(&mut self, _: kernel_core::window::Span) -> Result<Reduction, NativeError> {
        Err(NativeError::Bounds)
    }
}
