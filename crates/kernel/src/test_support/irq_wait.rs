//! Bounded fixture polling; success requires an observed IRQ delivery.
//! Finalize with IRQs masked so delivery cannot race the returned result.
pub fn wait_for_delivery(
    mut delivered: impl FnMut() -> bool,
    mut expired: impl FnMut() -> bool,
    quiesce: impl FnOnce(),
) -> bool {
    let observed = loop {
        if delivered() {
            break true;
        }
        if expired() {
            break false;
        }
        core::hint::spin_loop();
    };
    // The caller masks IRQs before the final observation. This closes both the
    // deadline-check race and the previous poll-return-to-mask window.
    quiesce();
    observed || delivered()
}

#[cfg(test)]
mod tests {
    use super::wait_for_delivery;
    #[test]
    fn already_delivered_irq_survives_an_expired_polling_deadline() {
        assert!(wait_for_delivery(
            || true,
            || panic!("delivery must be checked first"),
            || {},
        ));
    }
    #[test]
    fn delivery_published_during_deadline_check_is_observed() {
        let delivery = core::cell::Cell::new(false);
        assert!(wait_for_delivery(
            || delivery.get(),
            || {
                delivery.set(true);
                true
            },
            || {},
        ));
    }
    #[test]
    fn expired_without_delivery_is_rejected() {
        assert!(!wait_for_delivery(|| false, || true, || {}));
    }
    #[test]
    fn waiting_requires_actual_delivery_and_stops_at_expiration() {
        let polls = core::cell::Cell::new(0);
        assert!(!wait_for_delivery(
            || false,
            || {
                polls.set(polls.get() + 1);
                polls.get() == 3
            },
            || {},
        ));
        assert_eq!(polls.get(), 3);
    }
    #[test]
    fn delivery_between_expiration_and_irq_mask_is_not_discarded() {
        let delivery = core::cell::Cell::new(false);
        let masked = core::cell::Cell::new(false);
        assert!(wait_for_delivery(
            || delivery.get(),
            || true,
            || {
                delivery.set(true); // IRQ runs after the last poll, before masking.
                masked.set(true);
            },
        ));
        assert!(masked.get());
    }
}
