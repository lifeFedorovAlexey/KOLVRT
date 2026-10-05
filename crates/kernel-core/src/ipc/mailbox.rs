//! One owner-local wait per process, one retained wake and one acknowledgement.
//! Source Endpoint keeps the exact Wake until ack; sequence values are globally
//! nonwrapping WaitKey sequences, never process-slot generations or pointers.
use super::Error;
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Mailbox {
    state: AtomicU64,
}
impl Default for Mailbox {
    fn default() -> Self {
        Self::new()
    }
}
impl Mailbox {
    pub const fn new() -> Self {
        Self {
            state: AtomicU64::new(0),
        }
    }
    /// Multiple deferred source CPUs may publish the same retained Wake. Busy
    /// means keep the source record and retry later; no control work is dropped.
    /// True requests a target notification; false is an already published wake.
    /// Publish and source ack retirement require the same Endpoint exclusion.
    /// No copied Wake may be published after leaving that source permit: this
    /// excludes delayed publication after the acknowledged source was retired.
    pub fn publish(&self, sequence: u64) -> Result<bool, Error> {
        if sequence == 0 || sequence > u64::MAX >> 1 {
            return Err(Error::Invalid);
        }
        match self
            .state
            .compare_exchange(0, sequence << 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(true),
            Err(value) if value >> 1 == sequence => Ok(false),
            Err(_) => Err(Error::Busy),
        }
    }
    /// Only the fixed target owner interprets this against its exact saved wait
    /// reason. A mismatched/dead process is acknowledged without making READY.
    pub fn pending(&self) -> Option<u64> {
        let value = self.state.load(Ordering::Acquire);
        (value != 0 && value & 1 == 0).then_some(value >> 1)
    }
    /// The owner calls this only after applying or rejecting that exact wake.
    /// An outstanding ack prevents consuming another wake until source drainage.
    /// Exactly one target owner is required; source publishers never call consume.
    pub fn consume(&self, sequence: u64) -> Result<(), Error> {
        if sequence == 0 || sequence > u64::MAX >> 1 {
            return Err(Error::Invalid);
        }
        self.state
            .compare_exchange(
                sequence << 1,
                (sequence << 1) | 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .map(|_| ())
            .map_err(|value| {
                if value & 1 != 0 {
                    Error::Busy
                } else {
                    Error::Stale
                }
            })
    }
    /// Check against a retained source Wake before retiring it. The source must
    /// acknowledge Endpoint first, then remove this ack with finish_acknowledgement.
    pub fn acknowledged(&self, sequence: u64) -> bool {
        sequence != 0
            && sequence <= u64::MAX >> 1
            && self.state.load(Ordering::Acquire) == (sequence << 1) | 1
    }
    pub fn finish_acknowledgement(&self, sequence: u64) -> Result<(), Error> {
        if sequence == 0 || sequence > u64::MAX >> 1 {
            return Err(Error::Invalid);
        }
        self.state
            .compare_exchange((sequence << 1) | 1, 0, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| Error::Stale)
    }
    pub fn quiescent(&self) -> bool {
        self.state.load(Ordering::Acquire) == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wake_and_ack_saturation_retain_every_generation() {
        let mailbox = Mailbox::new();
        assert_eq!(mailbox.publish(0), Err(Error::Invalid));
        assert_eq!(mailbox.publish(1 << 63), Err(Error::Invalid));
        assert_eq!(mailbox.publish(1), Ok(true));
        assert_eq!(mailbox.publish(1), Ok(false));
        assert_eq!(mailbox.publish(2), Err(Error::Busy));
        assert_eq!(mailbox.consume(2), Err(Error::Stale));
        assert_eq!(mailbox.pending(), Some(1));
        mailbox.consume(1).unwrap();
        assert!(!mailbox.quiescent());
        assert_eq!(mailbox.publish(1), Ok(false));
        assert_eq!(mailbox.publish(2), Err(Error::Busy));
        assert_eq!(mailbox.consume(2), Err(Error::Busy));
        assert!(!mailbox.acknowledged(2));
        assert_eq!(mailbox.finish_acknowledgement(2), Err(Error::Stale));
        assert!(mailbox.acknowledged(1));
        mailbox.finish_acknowledgement(1).unwrap();
        assert_eq!(mailbox.publish(2), Ok(true));
        mailbox.consume(2).unwrap();
        mailbox.finish_acknowledgement(2).unwrap();
        assert!(mailbox.quiescent());
    }
    #[test]
    fn concurrent_duplicate_publish_and_delayed_owner_ack() {
        let mailbox = std::sync::Arc::new(Mailbox::new());
        for sequence in 1..=256 {
            std::thread::scope(|scope| {
                let first = &mailbox;
                let second = &mailbox;
                let a = scope.spawn(move || first.publish(sequence).unwrap());
                let b = scope.spawn(move || second.publish(sequence).unwrap());
                assert_ne!(a.join().unwrap(), b.join().unwrap());
            });
            assert_eq!(mailbox.pending(), Some(sequence));
            mailbox.consume(sequence).unwrap();
            assert!(mailbox.acknowledged(sequence));
            mailbox.finish_acknowledgement(sequence).unwrap();
            assert!(mailbox.quiescent());
        }
    }
}
