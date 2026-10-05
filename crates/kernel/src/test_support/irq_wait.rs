//! Bounded fixture polling; success requires an observed IRQ delivery.
//! Expiration stops waiting, but cannot erase a delivery published at the boundary.
pub fn wait_for_delivery(
    mut delivered: impl FnMut() -> bool,
    mut expired: impl FnMut() -> bool,
) -> bool {
    loop {
        if delivered() {
            return true;
        }
        if expired() {
            // An IRQ can publish delivery between the first observation and the
            // deadline check. Take the final observation before declaring failure.
            return delivered();
        }
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::wait_for_delivery;
    #[test]
    fn already_delivered_irq_survives_an_expired_polling_deadline() {
        assert!(wait_for_delivery(
            || true,
            || panic!("delivery must be checked first")
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
            }
        ));
    }
    #[test]
    fn expired_without_delivery_is_rejected() {
        assert!(!wait_for_delivery(|| false, || true));
    }
    #[test]
    fn waiting_requires_actual_delivery_and_stops_at_expiration() {
        let polls = core::cell::Cell::new(0);
        assert!(!wait_for_delivery(
            || false,
            || {
                polls.set(polls.get() + 1);
                polls.get() == 3
            }
        ));
        assert_eq!(polls.get(), 3);
    }
}
