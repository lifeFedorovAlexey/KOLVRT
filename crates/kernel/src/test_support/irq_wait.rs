//! Wait for an IRQ without a check-to-sleep lost wakeup.
//! Inspect the condition with IRQs masked; pending IRQs must wake the masked
//! wait, then be serviced before the next observation. Return masked.
pub fn wait_for_delivery(
    mut delivered: impl FnMut() -> bool,
    mut expired: impl FnMut() -> bool,
    mut mask: impl FnMut(),
    mut wait_and_service: impl FnMut(),
) -> bool {
    loop {
        mask();
        if delivered() {
            return true;
        }
        if expired() {
            return false;
        }
        wait_and_service();
    }
}

#[cfg(test)]
mod tests {
    use super::wait_for_delivery;
    use core::cell::Cell;

    #[test]
    fn delivery_before_masking_does_not_enter_sleep() {
        let delivered = Cell::new(false);
        let masked = Cell::new(false);
        assert!(wait_for_delivery(
            || {
                assert!(masked.get());
                delivered.get()
            },
            || panic!("delivery takes precedence over deadline"),
            || {
                delivered.set(true);
                masked.set(true);
            },
            || panic!("already serviced IRQ must not sleep"),
        ));
        assert!(masked.get());
    }

    #[test]
    fn single_irq_between_condition_and_sleep_remains_pending() {
        let masked = Cell::new(false);
        let pending = Cell::new(false);
        let delivered = Cell::new(false);
        let sleeps = Cell::new(0);
        assert!(wait_for_delivery(
            || {
                assert!(masked.get());
                delivered.get()
            },
            || {
                assert!(masked.get());
                // Inject the only source event after the negative condition
                // observation. Masking prevents its handler from consuming it
                // and disabling the one-shot source before WFI.
                pending.set(true);
                false
            },
            || masked.set(true),
            || {
                assert!(masked.get());
                assert!(pending.replace(false), "sleep lost its only wakeup");
                sleeps.set(sleeps.get() + 1);
                masked.set(false);
                delivered.set(true); // service the pending IRQ after wake
                masked.set(true);
            },
        ));
        assert_eq!(sleeps.get(), 1);
        assert!(masked.get());
    }

    #[test]
    fn already_delivered_irq_survives_expired_deadline() {
        assert!(wait_for_delivery(
            || true,
            || panic!("checked too late"),
            || {},
            || panic!("sleep")
        ));
    }

    #[test]
    fn expired_without_delivery_is_rejected_without_sleep() {
        assert!(!wait_for_delivery(
            || false,
            || true,
            || {},
            || panic!("sleep after expiry")
        ));
    }

    #[test]
    fn spurious_wakes_do_not_establish_delivery() {
        let wakes = Cell::new(0);
        assert!(!wait_for_delivery(
            || false,
            || wakes.get() == 3,
            || {},
            || wakes.set(wakes.get() + 1),
        ));
        assert_eq!(wakes.get(), 3);
    }

    #[test]
    fn pending_irq_at_deadline_is_not_reported_as_delivered() {
        let pending = Cell::new(false);
        assert!(!wait_for_delivery(
            || false,
            || {
                pending.set(true);
                true
            },
            || {},
            || panic!("expired wait must not service a pending source as success"),
        ));
        assert!(pending.get());
    }
}
