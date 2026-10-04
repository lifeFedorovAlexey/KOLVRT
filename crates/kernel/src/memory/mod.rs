use crate::platform::config;
use crate::{arch::aarch64 as cpu, sync::Lock};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr::null_mut,
    sync::atomic::{AtomicUsize, Ordering},
};
use cpu::page;
use kernel_core::{memory::Pool, platform::Description};
const PAGE_SIZE: usize = config::PAGE_BYTES;
const TABLE_ENTRIES: usize = 512;
const BLOCK_SHIFT: u32 = 21;
const BLOCK_SIZE: usize = 1 << BLOCK_SHIFT;
const RAM_PAGE_TABLES: usize = config::RAM_L3_TABLES;
const HEAP_PAGES: usize = 16;
pub(crate) const HEAP_BYTES: usize = HEAP_PAGES * PAGE_SIZE;
pub(crate) const HEAP_UNIT_BYTES: usize = 64;
const HEAP_BITMAP_WORDS: usize = (HEAP_BYTES / HEAP_UNIT_BYTES).div_ceil(u64::BITS as usize);
const DEVICE_ROOT_INDEX: usize = 0;
const RAM_ROOT_INDEX: usize = 1;
const DYNAMIC_ROOT_INDEX: usize = 2;
#[repr(C, align(4096))]
struct Table([u64; TABLE_ENTRIES]);
impl Table {
    const ZERO: Self = Self([0; TABLE_ENTRIES]);
}
#[repr(C)]
struct Tables {
    root: Table,
    devices: Table,
    ram: Table,
    ram_pages: [Table; RAM_PAGE_TABLES],
    test: Table,
    test_pages: Table,
}
static TABLES: Lock<Tables> = Lock::new(Tables {
    root: Table::ZERO,
    devices: Table::ZERO,
    ram: Table::ZERO,
    ram_pages: [const { Table::ZERO }; RAM_PAGE_TABLES],
    test: Table::ZERO,
    test_pages: Table::ZERO,
});
const BLOCK_DESCRIPTOR: u64 = 1;
pub const DYNAMIC_BASE: usize = config::DYNAMIC_BASE;
static PHYSICAL_OWNER: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
static RAM_START: AtomicUsize = AtomicUsize::new(0);
static RAM_END: AtomicUsize = AtomicUsize::new(0);
unsafe extern "C" {
    static __kernel_start: u8;
    static __text_end: u8;
    static __rodata_end: u8;
    static __kernel_end: u8;
}
pub fn kernel_bounds() -> (usize, usize) {
    (
        &raw const __kernel_start as usize,
        &raw const __kernel_end as usize,
    )
}
pub struct Physical {
    pool: Pool<{ config::PHYSICAL_BITMAP_WORDS }>,
    base: usize,
    owner: core::marker::PhantomData<*mut ()>,
}
impl Physical {
    pub fn new(d: &Description) -> Self {
        crate::percpu::primary_only();
        assert!(
            PHYSICAL_OWNER
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            "physical pool already owned"
        );
        let mut p = Self {
            pool: Pool::empty(),
            base: d.ram.base as usize,
            owner: core::marker::PhantomData,
        };
        p.pool.initialize(d.ram.size as usize / PAGE_SIZE).unwrap();
        let (_, end) = kernel_bounds();
        p.reserve(p.base, end - p.base);
        for r in &d.reserved[..d.reserved_count] {
            let start = r.base.max(d.ram.base);
            let stop = r.end().unwrap().min(d.ram.end().unwrap());
            if stop > start {
                p.reserve(start as usize, (stop - start) as usize);
            }
        }
        p
    }
    fn reserve(&mut self, start: usize, len: usize) {
        let first = (start - self.base) / PAGE_SIZE;
        let end = (start - self.base + len).div_ceil(PAGE_SIZE);
        self.pool.reserve(first, end - first).unwrap();
    }
    pub fn allocate(&mut self, count: usize, align: usize) -> Option<Frame> {
        crate::percpu::primary_only();
        self.pool.allocate(count, align).ok().map(|i| {
            let address = self.base + i * PAGE_SIZE;
            // SAFETY: INV-FRAME: CPU0-only allocation, exclusively claimed RAM; prior retirement acknowledged before pool reuse, no IRQ reader.
            unsafe {
                core::ptr::write_bytes(address as *mut u8, 0, count * PAGE_SIZE);
            }
            Frame {
                address,
                count,
                owner: core::marker::PhantomData,
            }
        })
    }
    /// Retain and zero a known free physical extent so a same-VA TLB test can
    /// keep the previous process backing unavailable across ASID reuse.
    pub fn pin_extent_at(&mut self, address: usize, count: usize) -> Option<Frame> {
        crate::percpu::primary_only();
        if count == 0 || address < self.base || !(address - self.base).is_multiple_of(PAGE_SIZE) {
            return None;
        }
        let index = (address - self.base) / PAGE_SIZE;
        self.pool.allocate_at(index, count).ok()?;
        // SAFETY: INV-FRAME: exact free pool extent claimed above; zero before use.
        unsafe {
            core::ptr::write_bytes(address as *mut u8, 0, count * PAGE_SIZE);
        }
        Some(Frame {
            address,
            count,
            owner: core::marker::PhantomData,
        })
    }
    /// Consumes ownership only after the caller has unmapped and invalidated all aliases.
    pub fn release(&mut self, frame: Frame) {
        crate::percpu::primary_only();
        assert!(self.reclaimable(&frame), "retiring frame release");
        assert!(
            USER_CHARGES.iter().all(|charge| {
                let address = charge.load(Ordering::Acquire);
                address == 0
                    || !(frame.address..frame.address + frame.count * PAGE_SIZE).contains(&address)
            }),
            "user address space retains frame"
        );
        let t = TABLES.lock();
        assert!(
            !t.test_pages
                .0
                .iter()
                .any(|&pte| pte & page::TABLE_OR_PAGE == page::TABLE_OR_PAGE
                    && (frame.address as u64..(frame.address + frame.count * PAGE_SIZE) as u64)
                        .contains(&(pte & page::OUTPUT_ADDRESS_MASK))),
            "mapped frame release"
        );
        drop(t);
        self.pool
            .release((frame.address - self.base) / PAGE_SIZE, frame.count)
            .unwrap();
    }
    pub fn available(&self) -> usize {
        self.pool.available()
    }
    pub fn reclaimable(&self, frame: &Frame) -> bool {
        let retiring = RETIRING_FRAME.load(Ordering::Acquire);
        retiring == 0
            || !(frame.address..frame.address + frame.count * PAGE_SIZE).contains(&retiring)
    }
}
pub struct Frame {
    address: usize,
    count: usize,
    owner: core::marker::PhantomData<*mut ()>,
}
impl Frame {
    #[cfg(feature = "kernel-tests")]
    pub fn address(&self) -> usize {
        self.address
    }
}
pub fn initialize_mmu(d: &Description) {
    crate::percpu::primary_only();
    RAM_START.store(d.ram.base as usize, Ordering::Release);
    RAM_END.store(d.ram.end().unwrap() as usize, Ordering::Release);
    let mut t = TABLES.lock();
    t.root.0[DEVICE_ROOT_INDEX] = (&raw const t.devices as u64) | page::TABLE_OR_PAGE;
    t.root.0[RAM_ROOT_INDEX] = (&raw const t.ram as u64) | page::TABLE_OR_PAGE;
    t.root.0[DYNAMIC_ROOT_INDEX] = (&raw const t.test as u64) | page::TABLE_OR_PAGE;
    t.test.0[0] = (&raw const t.test_pages as u64) | page::TABLE_OR_PAGE;
    for region in [d.uart, d.distributor, d.redistributor] {
        let first = region.base as usize >> BLOCK_SHIFT;
        let last = (region.end().unwrap() as usize - 1) >> BLOCK_SHIFT;
        for i in first..=last {
            assert!(i < TABLE_ENTRIES);
            t.devices.0[i] = ((i as u64) << BLOCK_SHIFT)
                | BLOCK_DESCRIPTOR
                | page::ACCESS_FLAG
                | page::PRIVILEGED_EXECUTE_NEVER
                | page::USER_EXECUTE_NEVER;
        }
    }
    let text_end = &raw const __text_end as u64;
    let ro_end = &raw const __rodata_end as u64;
    let (start, _) = kernel_bounds();
    for i in 0..RAM_PAGE_TABLES {
        t.ram.0[i] = (&raw const t.ram_pages[i] as u64) | page::TABLE_OR_PAGE;
        for j in 0..TABLE_ENTRIES {
            let pa = d.ram.base + ((i * TABLE_ENTRIES + j) * PAGE_SIZE) as u64;
            let executable = pa >= start as u64 && pa < text_end;
            let writable = pa < start as u64 || pa >= ro_end;
            t.ram_pages[i].0[j] = cpu::page::descriptor(pa, writable, executable).unwrap();
        }
    }
    cpu::enable_mmu(&raw const t.root as u64);
}
pub fn table_root() -> u64 {
    crate::percpu::primary_only();
    let t = TABLES.lock();
    &raw const t.root as u64
}
static RETIRING_FRAME: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "kernel-tests")]
pub fn retirement_pending() -> bool {
    RETIRING_FRAME.load(Ordering::Acquire) != 0
}
pub struct Mapping<'a> {
    frame: &'a Frame,
    index: usize,
    writable: bool,
}
pub fn map(
    frame: &Frame,
    va: usize,
    writable: bool,
    executable: bool,
) -> Result<Mapping<'_>, kernel_core::memory::Error> {
    crate::percpu::primary_only();
    let pa = frame.address;
    if RETIRING_FRAME.load(Ordering::Acquire) != 0
        || USER_EXECUTION_ACTIVE.load(Ordering::Acquire) != 0
    {
        return Err(kernel_core::memory::Error::Occupied);
    }
    let descriptor = cpu::page::descriptor(pa as u64, writable, executable)?;
    let (start, end) = kernel_bounds();
    if !(RAM_START.load(Ordering::Acquire)..RAM_END.load(Ordering::Acquire)).contains(&pa)
        || (start..end).contains(&pa)
        || executable
        || !(DYNAMIC_BASE..DYNAMIC_BASE + BLOCK_SIZE).contains(&va)
        || !va.is_multiple_of(PAGE_SIZE)
    {
        return Err(kernel_core::memory::Error::Invalid);
    }
    let index = (va - DYNAMIC_BASE) / PAGE_SIZE;
    let mut t = TABLES.lock();
    if t.test_pages.0[index] != 0 {
        return Err(kernel_core::memory::Error::Occupied);
    }
    t.test_pages.0[index] = descriptor;
    drop(t);
    let generation = crate::smp::request_invalidation();
    crate::smp::finish_invalidation(generation);
    Ok(Mapping {
        frame,
        index,
        writable,
    })
}
impl<'a> Mapping<'a> {
    pub fn address(&self) -> usize {
        DYNAMIC_BASE + self.index * PAGE_SIZE
    }
    pub fn unmap(self) {
        drop(self.retire());
    }
    pub fn retire(self) -> Retirement<'a> {
        let frame = self.frame;
        let index = self.index;
        core::mem::forget(self);
        retire_mapping(frame, index)
    }
    pub fn read_word(&self) -> u64 {
        crate::percpu::primary_only();
        // SAFETY: INV-FRAME: CPU0 access through retained guard; remote immutable reader is explicitly coordinated, IRQ never accesses mappings.
        unsafe { core::ptr::read_volatile(self.address() as *const u64) }
    }
    pub fn write_word(&mut self, value: u64) -> Result<(), kernel_core::memory::Error> {
        crate::percpu::primary_only();
        if !self.writable {
            return Err(kernel_core::memory::Error::Invalid);
        }
        // SAFETY: INV-FRAME: CPU0-owned write; caller must complete before publishing a remote read; no shared mutable Rust alias or IRQ access.
        unsafe {
            core::ptr::write_volatile(self.address() as *mut u64, value);
        }
        Ok(())
    }
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        drop(retire_mapping(self.frame, self.index));
    }
}
fn retire_mapping(frame: &Frame, index: usize) -> Retirement<'_> {
    crate::percpu::primary_only();
    assert_eq!(
        RETIRING_FRAME.compare_exchange(0, frame.address, Ordering::AcqRel, Ordering::Acquire),
        Ok(0),
        "one pending frame retirement"
    );
    let mut t = TABLES.lock();
    assert_eq!(
        t.test_pages.0[index] & page::OUTPUT_ADDRESS_MASK,
        frame.address as u64
    );
    t.test_pages.0[index] = 0;
    drop(t);
    let generation = crate::smp::request_invalidation();
    Retirement { frame, generation }
}
pub struct Retirement<'a> {
    frame: &'a Frame,
    generation: u64,
}
impl Retirement<'_> {
    #[cfg(feature = "kernel-tests")]
    pub fn acknowledged(&self) -> bool {
        crate::smp::acknowledged(self.generation)
    }
}
impl Drop for Retirement<'_> {
    fn drop(&mut self) {
        crate::smp::finish_invalidation(self.generation);
        assert_eq!(RETIRING_FRAME.swap(0, Ordering::AcqRel), self.frame.address);
    }
}
static HEAP_BASE: AtomicUsize = AtomicUsize::new(0);
pub const USER_BASE: usize = 0x2000_0000;
pub const USER_CODE: usize = USER_BASE;
pub const USER_DATA: usize = USER_BASE + PAGE_SIZE;
pub const USER_GUARD: usize = USER_BASE + 2 * PAGE_SIZE;
const USER_STACK_PAGES: usize = config::USER_STACK_PAGES;
pub const USER_STACK_TOP: usize = USER_BASE + (USER_STACK_INDEX + USER_STACK_PAGES) * PAGE_SIZE;
const USER_STACK_INDEX: usize = 3;
const USER_ALIAS_INDEX: usize = 16;
const USER_ROOT_PAGE: usize = 0;
const USER_DEVICES_PAGE: usize = 1;
const USER_LEAVES_PAGE: usize = 2;
const USER_CODE_PAGE: usize = 3;
const USER_DATA_PAGE: usize = 4;
pub const USER_DATA_FRAME_OFFSET: usize = USER_DATA_PAGE * PAGE_SIZE;
const USER_STACK_PAGE: usize = 5;
pub const USER_SPACE_PAGES: usize = USER_STACK_PAGE + USER_STACK_PAGES;
static USER_CHARGES: [AtomicUsize; crate::process::CAPACITY] =
    [const { AtomicUsize::new(0) }; crate::process::CAPACITY];
pub static USER_EXECUTION_ACTIVE: AtomicUsize = AtomicUsize::new(0);
/// Linear private space owner for a dynamic process, with no self-referential
/// borrow. The persistent charge protects even a forgotten owner. No automatic
/// release can bypass explicit scheduler detachment and Physical ownership.
pub struct OwnedUserSpace {
    frame: Frame,
    id: usize,
    lease: crate::asid::Lease,
}
static USER_GENERATIONS: [core::sync::atomic::AtomicU64; crate::process::CAPACITY] =
    [const { core::sync::atomic::AtomicU64::new(0) }; crate::process::CAPACITY];
pub(crate) fn current_space(slot: usize, generation: u64, root: u64) -> bool {
    slot < USER_CHARGES.len()
        && generation != 0
        && root != 0
        && USER_CHARGES[slot].load(Ordering::Acquire) as u64 == root
        && USER_GENERATIONS[slot].load(Ordering::Acquire) == generation
}
#[cfg(feature = "kernel-tests")]
pub(crate) fn live_space_generation(slot: usize, generation: u64) -> bool {
    slot < USER_CHARGES.len()
        && generation != 0
        && USER_GENERATIONS[slot].load(Ordering::Acquire) == generation
        && USER_CHARGES[slot].load(Ordering::Acquire) != 0
}
impl OwnedUserSpace {
    pub fn slot(&self) -> usize {
        self.id
    }
    pub fn new(
        frame: Frame,
        identity: kernel_core::process::ProcessId,
        image: &[u8],
        entry: usize,
    ) -> Self {
        let id = identity.slot();
        let space = UserSpace::image(&frame, id, image, entry);
        let lease = crate::asid::allocate(id, id / config::USER_PROCESSES_PER_CPU);
        USER_GENERATIONS[id].store(identity.generation(), Ordering::Release);
        // Transfer the same charge, not another mapping or allocation. All
        // construction borrows end here; the owned Frame stays live until reclaim.
        core::mem::forget(space);
        Self { frame, id, lease }
    }
    pub fn root(&self) -> u64 {
        self.frame.address as u64
    }
    /// Number of physical frames owned by this process space, including its
    /// root, fixed mappings and immutable image pages.
    #[cfg(feature = "machine-events")]
    pub fn resident_pages(&self) -> usize {
        self.frame.count
    }
    pub fn data_address(&self) -> usize {
        self.frame.address + USER_DATA_PAGE * PAGE_SIZE
    }
    pub fn lease(&self) -> crate::asid::Lease {
        self.lease
    }
    pub fn reclaim(self, physical: &mut Physical) {
        // Only kernel-internal lifecycle code owns this value. The caller checked
        // acquired scheduler unlink; the existing guard enforces CPU quiescence.
        drop(UserSpace {
            frame: &self.frame,
            id: self.id,
        });
        USER_GENERATIONS[self.id].store(0, Ordering::Release);
        crate::asid::release(self.lease, false);
        physical.release(self.frame);
    }
    /// Roll back a fully constructed but never published root.
    pub fn rollback(self, physical: &mut Physical) {
        drop(UserSpace {
            frame: &self.frame,
            id: self.id,
        });
        USER_GENERATIONS[self.id].store(0, Ordering::Release);
        crate::asid::release(self.lease, true);
        physical.release(self.frame);
    }
}
/// CPU0 owns setup/reclamation. Borrow plus persistent charge protects forgotten guards.
/// The scheduler may publish a root only under its separate unsafe lifetime contract.
pub struct UserSpace<'a> {
    frame: &'a Frame,
    id: usize,
}
impl<'a> UserSpace<'a> {
    #[cfg(feature = "user-retirement-negative")]
    pub fn new(frame: &'a Frame, id: usize, image: &[u8]) -> Self {
        Self::image(frame, id, image, USER_CODE)
    }
    fn image(frame: &'a Frame, id: usize, image: &[u8], entry: usize) -> Self {
        crate::percpu::primary_only();
        assert_eq!(USER_EXECUTION_ACTIVE.load(Ordering::Acquire), 0);
        let payload = entry == config::USER_PAYLOAD_BASE;
        let image_pages = image.len().div_ceil(PAGE_SIZE);
        assert_eq!(
            frame.count,
            USER_SPACE_PAGES + if payload { image_pages } else { 0 }
        );
        assert!(id < USER_CHARGES.len() && !image.is_empty());
        assert!(if payload {
            image.len() <= config::USER_PAYLOAD_BYTES
        } else {
            image.len() <= PAGE_SIZE
        });
        let code_page = if payload {
            USER_SPACE_PAGES
        } else {
            USER_CODE_PAGE
        };
        let code_index = (entry - USER_BASE) / PAGE_SIZE;
        assert_eq!(
            USER_CHARGES[id].compare_exchange(
                0,
                frame.address,
                Ordering::AcqRel,
                Ordering::Acquire
            ),
            Ok(0)
        );
        let native = TABLES.lock();
        // SAFETY: INV-USER-SPACE: exclusive zeroed contiguous allocation, checked extents;
        // private page-table storage, no published root or user execution during construction.
        unsafe {
            let root = &mut *((frame.address + USER_ROOT_PAGE * PAGE_SIZE) as *mut Table);
            let devices = &mut *((frame.address + USER_DEVICES_PAGE * PAGE_SIZE) as *mut Table);
            let leaves = &mut *((frame.address + USER_LEAVES_PAGE * PAGE_SIZE) as *mut Table);
            root.0.copy_from_slice(&native.root.0);
            devices.0.copy_from_slice(&native.devices.0);
            root.0[DEVICE_ROOT_INDEX] = (devices as *mut Table as u64) | page::TABLE_OR_PAGE;
            assert_eq!(devices.0[USER_BASE >> BLOCK_SHIFT], 0);
            devices.0[USER_BASE >> BLOCK_SHIFT] =
                (leaves as *mut Table as u64) | page::TABLE_OR_PAGE;
            for offset in 0..image_pages {
                leaves.0[code_index + offset] = page::user_descriptor(
                    (frame.address + (code_page + offset) * PAGE_SIZE) as u64,
                    false,
                    true,
                )
                .unwrap();
            }
            leaves.0[1] = page::user_descriptor(
                (frame.address + USER_DATA_PAGE * PAGE_SIZE) as u64,
                true,
                false,
            )
            .unwrap();
            for offset in 0..USER_STACK_PAGES {
                leaves.0[USER_STACK_INDEX + offset] = page::user_descriptor(
                    (frame.address + (USER_STACK_PAGE + offset) * PAGE_SIZE) as u64,
                    true,
                    false,
                )
                .unwrap();
            }
            leaves.0[USER_ALIAS_INDEX + id] = leaves.0[1];
            core::ptr::copy_nonoverlapping(
                image.as_ptr(),
                (frame.address + code_page * PAGE_SIZE) as *mut u8,
                image.len(),
            );
        }
        drop(native);
        cpu::publish_instructions(frame.address, frame.address + frame.count * PAGE_SIZE);
        Self { frame, id }
    }
    pub fn alias(id: usize) -> usize {
        USER_BASE + (USER_ALIAS_INDEX + id) * PAGE_SIZE
    }
}
impl Drop for UserSpace<'_> {
    fn drop(&mut self) {
        crate::percpu::primary_only();
        assert_eq!(
            USER_EXECUTION_ACTIVE.load(Ordering::Acquire),
            0,
            "active user space retirement"
        );
        // Execution admission has ended; every participating CPU restored native root and
        // completed ASID retirement (or full-flush fallback) before its release completion.
        // Charge is last to go.
        assert_eq!(
            USER_CHARGES[self.id].swap(0, Ordering::AcqRel),
            self.frame.address
        );
    }
}
static HEAP_PLAN: Lock<Pool<HEAP_BITMAP_WORDS>> = Lock::new(Pool::empty());
pub fn initialize_heap(p: &mut Physical) {
    let frame = p
        .allocate(HEAP_PAGES, HEAP_PAGES)
        .expect("heap physical exhaustion");
    assert_eq!(frame.count, HEAP_PAGES);
    HEAP_PLAN
        .lock()
        .initialize(HEAP_BYTES / HEAP_UNIT_BYTES)
        .unwrap();
    // INV-HEAP: allocation permanently transferred to heap, not returned to physical pool.
    HEAP_BASE.store(frame.address, Ordering::Release);
}
struct Heap;
// SAFETY: INV-HEAP: reserved 64 KiB RAM, disjoint live ranges from locked plan; caller obeys GlobalAlloc.
unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = HEAP_BASE.load(Ordering::Acquire);
        if base == 0 || layout.align() > HEAP_BYTES {
            return null_mut();
        }
        let count = layout.size().max(1).div_ceil(HEAP_UNIT_BYTES);
        let align = layout.align().max(HEAP_UNIT_BYTES) / HEAP_UNIT_BYTES;
        HEAP_PLAN
            .lock()
            .allocate(count, align)
            .map_or(null_mut(), |i| (base + i * HEAP_UNIT_BYTES) as *mut u8)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let base = HEAP_BASE.load(Ordering::Acquire);
        let address = ptr as usize;
        assert!(
            address >= base
                && address < base + HEAP_BYTES
                && (address - base).is_multiple_of(HEAP_UNIT_BYTES)
        );
        HEAP_PLAN
            .lock()
            .release(
                (address - base) / HEAP_UNIT_BYTES,
                layout.size().max(1).div_ceil(HEAP_UNIT_BYTES),
            )
            .unwrap();
    }
}
#[global_allocator]
static HEAP: Heap = Heap;
