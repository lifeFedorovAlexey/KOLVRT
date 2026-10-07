use kernel_core::{
    domain::Limits,
    supervision::{Error, Grant, validate_grants},
};
fn grants() -> [Grant<'static, ()>; 3] {
    core::array::from_fn(|i| Grant {
        image: &[1],
        image_format: (),
        send_to: if i == 1 { Some(0) } else { None },
        owner: i % 2,
        limits: Limits {
            memory_pages: 64,
            handles: 8,
            queue: 1,
            requests: 4,
            endpoints: 1,
        },
        instances: if i == 0 { 4 } else { 1 },
    })
}
#[test]
fn immutable_service_client_manifest_accepts_existing_bounds() {
    let input = grants();
    assert_eq!(validate_grants(&input), Ok(()));
    for credits in 1..=8 {
        let mut input = input;
        input[0].instances = credits;
        assert_eq!(validate_grants(&input), Ok(()));
    }
}
#[test]
fn manifest_rejects_empty_images_and_invalid_instance_credits() {
    let mut input = grants();
    input[0].image = &[];
    assert_eq!(validate_grants(&input), Err(Error::EmptyImage));
    for credits in [0, 9, usize::MAX] {
        let mut input = grants();
        input[0].instances = credits;
        assert_eq!(validate_grants(&input), Err(Error::InstanceCredits));
    }
}
#[test]
fn manifest_rejects_self_or_missing_send_destination_without_mutating_grants() {
    for target in [1, 3, usize::MAX] {
        let mut input = grants();
        input[1].send_to = Some(target);
        assert_eq!(validate_grants(&input), Err(Error::SendEdge));
        assert_eq!(input[1].send_to, Some(target));
        assert_eq!(input[0].instances, 4);
    }
}
