#[path = "../../kernel/src/sync/mod.rs"]
mod kernel_sync;
#[path = "../../kernel/src/arch/aarch64/page.rs"]
mod page;
use std::{sync::Arc, thread};
const WORKERS: usize = 4;
const UPDATES_PER_WORKER: usize = 2000;
#[test]
fn actual_concurrent_publication_and_exclusion() {
    let lock = Arc::new(kernel_sync::Lock::new((0u64, 0u64)));
    let mut threads = Vec::new();
    for _ in 0..WORKERS {
        let lock = Arc::clone(&lock);
        threads.push(thread::spawn(move || {
            for _ in 0..UPDATES_PER_WORKER {
                let mut g = lock.lock();
                assert_eq!(g.0, g.1);
                g.0 += 1;
                g.1 = g.0;
            }
        }));
    }
    for t in threads {
        t.join().unwrap();
    }
    let expected = (WORKERS * UPDATES_PER_WORKER) as u64;
    assert_eq!(*lock.lock(), (expected, expected));
}
