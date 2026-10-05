//! Endpoint-local syscall/deferred exclusion, never scheduler storage or IRQ work.
//! Unlike ordinary Lock, this permit never spins or runs a user-copy callback.
use crate::{cpu, percpu};
use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{AtomicBool, Ordering},
};
use kernel_core::ipc::{ENDPOINTS, Endpoint, Error, Reference};

struct Cell {
    held: AtomicBool,
    value: UnsafeCell<Option<Endpoint>>,
}
impl Cell {
    const fn new() -> Self {
        Self {
            held: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }
    fn acquire(&self) -> Result<Permit<'_>, Error> {
        assert!(
            cpu::irq_masked(),
            "IPC continuation must exclude local preemption"
        );
        if percpu::current().scheduler_borrow.load(Ordering::Acquire) {
            crate::scheduler::reject_ipc("StorageInsideScheduler");
        }
        crate::sync::assert_scheduler_unlocked();
        self.held
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .map_err(|_| Error::Busy)?;
        assert_eq!(
            percpu::current()
                .ordinary_locks
                .fetch_add(1, Ordering::AcqRel),
            0
        );
        Ok(Permit {
            cell: self,
            _local: PhantomData,
        })
    }
}
// SAFETY: INV-IPC-STORAGE: immutable Cell addresses, one acquired nonblocking
// endpoint-local permit for every dereference. Masked owning continuation only;
// IRQ publishes atomics and never accesses value. No scheduler scope, copy,
// external call, sleep or context switch spans the permit. HRTB excludes escaped
// storage borrows. A retained exact Reference excludes generation reuse, while
// removal additionally requires actual request/wait/mailbox/reference quiescence.
unsafe impl Sync for Cell {}
static CELLS: [Cell; ENDPOINTS] = [const { Cell::new() }; ENDPOINTS];
struct Permit<'a> {
    cell: &'a Cell,
    _local: PhantomData<*mut ()>,
}
impl Permit<'_> {
    fn edit<R>(&mut self, operation: impl for<'a> FnOnce(&'a mut Option<Endpoint>) -> R) -> R {
        // SAFETY: INV-IPC-STORAGE: this non-Send permit owns the acquired held
        // bit, local IRQ exclusion and ordinary-lock marker; no reference can
        // leave the higher-ranked callback or survive permit release.
        unsafe { operation(&mut *self.cell.value.get()) }
    }
}
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        assert_eq!(
            percpu::current()
                .ordinary_locks
                .fetch_sub(1, Ordering::AcqRel),
            1
        );
        self.cell.held.store(false, Ordering::Release);
    }
}
pub(super) fn create(
    domain: kernel_core::domain::Reference,
    cpu: usize,
    capacity: usize,
) -> Result<Reference, Error> {
    percpu::primary_only();
    assert_eq!(
        crate::memory::USER_EXECUTION_ACTIVE.load(Ordering::Acquire),
        0
    );
    let mut endpoint = Endpoint::create(domain, cpu, capacity)?;
    let cell = &CELLS[endpoint.reference().id().slot()];
    let mut permit = match cell.acquire() {
        Ok(permit) => permit,
        Err(error) => {
            endpoint.shutdown();
            return Err(error);
        }
    };
    let reference = match endpoint.reference().try_clone() {
        Ok(reference) => reference,
        Err(error) => {
            endpoint.shutdown();
            return Err(error);
        }
    };
    permit.edit(|slot| {
        assert!(
            slot.is_none(),
            "new endpoint identity requires vacant storage"
        );
        *slot = Some(endpoint);
    });
    Ok(reference)
}
pub(super) fn with<R>(
    reference: &Reference,
    operation: impl for<'a> FnOnce(&'a mut Endpoint) -> R,
) -> Result<R, Error> {
    let mut permit = CELLS[reference.id().slot()].acquire()?;
    permit.edit(|slot| {
        let endpoint = slot.as_mut().ok_or(Error::Stale)?;
        if endpoint.reference().id() != reference.id() {
            return Err(Error::Stale);
        }
        Ok(operation(endpoint))
    })
}
/// No endpoint selector comes from EL0. A globally unique receipt is resolved
/// only through its exact requester, including process generation. A skipped
/// busy cell prevents a false Stale response; the caller can retry Busy.
pub(super) fn receipt(
    caller: kernel_core::process::ProcessId,
    value: u64,
) -> Result<(Reference, kernel_core::ipc::RequestId), Error> {
    let mut busy = false;
    for cell in &CELLS {
        let mut permit = match cell.acquire() {
            Ok(permit) => permit,
            Err(Error::Busy) => {
                busy = true;
                continue;
            }
            Err(error) => return Err(error),
        };
        let found = permit.edit(|slot| {
            let Some(endpoint) = slot else {
                return Ok(None);
            };
            match endpoint.receipt(caller, value) {
                Ok(id) => Ok(Some((endpoint.reference().try_clone()?, id))),
                Err(Error::Stale) => Ok(None),
                Err(error) => Err(error),
            }
        })?;
        if let Some(found) = found {
            return Ok(found);
        }
    }
    Err(if busy { Error::Busy } else { Error::Stale })
}
/// Bounded deferred scan. Completion/IRQ never invokes this from a storage scope.
pub(super) fn each(mut operation: impl FnMut(&mut Endpoint)) -> bool {
    let mut complete = true;
    for cell in &CELLS {
        if let Ok(mut permit) = cell.acquire() {
            let rejected = permit.edit(|slot| {
                if let Some(endpoint) = slot.as_mut() {
                    operation(endpoint);
                }
                if slot.as_ref().is_some_and(Endpoint::reclaimable) {
                    let endpoint = slot.as_ref().expect("reclaimable endpoint exists");
                    if endpoint.outstanding() != 0
                        || endpoint.waiter_count() != 0
                        || endpoint.reference().references() != 1
                    {
                        return true;
                    }
                    // Final owner consumes the endpoint charge and control lease
                    // once, only after every retained request/wake/reference drains.
                    *slot = None;
                }
                false
            });
            drop(permit);
            if rejected {
                crate::scheduler::reject_ipc("EndpointNotQuiescent");
            }
        } else {
            complete = false;
        }
    }
    complete
}
