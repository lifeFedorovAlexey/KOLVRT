//! Concrete native IPC integration. Protected object work uses split continuations.
pub(crate) mod deferred;
pub(crate) mod native;
mod storage;
use kernel_core::ipc::{Error, Reference};

/// Trusted bootstrap creates a bound concrete endpoint, never an ambient EL0 issuer.
pub(crate) fn create(
    domain: kernel_core::domain::Reference,
    service_cpu: usize,
    capacity: usize,
) -> Result<Reference, Error> {
    if service_cpu >= crate::platform::config::ACTIVE_CPUS
        || domain.owner().slot() / crate::scheduler::TASKS != service_cpu
    {
        return Err(Error::Denied);
    }
    storage::create(domain, service_cpu, capacity)
}
pub(crate) fn discard(reference: &Reference) {
    storage::with(reference, |endpoint| endpoint.shutdown()).expect("exclusive bootstrap rollback");
}
pub(crate) fn reap() {
    assert!(
        deferred::mailboxes_quiescent(),
        "IPC mailbox retirement requires actual quiescence"
    );
    crate::percpu::primary_only();
    assert_eq!(
        crate::memory::USER_EXECUTION_ACTIVE.load(core::sync::atomic::Ordering::Acquire),
        0
    );
    assert!(
        storage::each(|_| {}),
        "quiescent bootstrap endpoint permit busy"
    );
}
