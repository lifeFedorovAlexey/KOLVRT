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
}
impl Physical {
    pub fn new(d: &Description) -> Self {
        assert!(
            PHYSICAL_OWNER
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok(),
            "physical pool already owned"
        );
        let mut p = Self {
            pool: Pool::empty(),
            base: d.ram.base as usize,
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
        self.pool.allocate(count, align).ok().map(|i| {
            let address = self.base + i * PAGE_SIZE;
            // SAFETY: INV-FRAME: exclusively claimed ordinary RAM, outside reserved regions; no published mapping or IRQ reader.
            unsafe {
                core::ptr::write_bytes(address as *mut u8, 0, count * PAGE_SIZE);
            }
            Frame { address, count }
        })
    }
    /// Consumes ownership only after the caller has unmapped and invalidated all aliases.
    pub fn release(&mut self, frame: Frame) {
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
}
pub struct Frame {
    address: usize,
    count: usize,
}
impl Frame {
    #[cfg(feature = "kernel-tests")]
    pub fn address(&self) -> usize {
        self.address
    }
}
pub fn initialize_mmu(d: &Description) {
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
    let pa = frame.address;
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
    cpu::invalidate();
    Ok(Mapping {
        frame,
        index,
        writable,
    })
}
impl Mapping<'_> {
    pub fn address(&self) -> usize {
        DYNAMIC_BASE + self.index * PAGE_SIZE
    }
    pub fn unmap(self) {
        drop(self);
    }
    pub fn read_word(&self) -> u64 {
        // SAFETY: INV-FRAME: owned initialized RAM, mapping retained by guard; no other active CPU or IRQ accesses it.
        unsafe { core::ptr::read_volatile(self.address() as *const u64) }
    }
    pub fn write_word(&mut self, value: u64) -> Result<(), kernel_core::memory::Error> {
        if !self.writable {
            return Err(kernel_core::memory::Error::Invalid);
        }
        // SAFETY: INV-FRAME: retained writable aligned mapping, single CPU; volatile access creates no borrowed alias.
        unsafe {
            core::ptr::write_volatile(self.address() as *mut u64, value);
        }
        Ok(())
    }
}
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        let mut t = TABLES.lock();
        assert_eq!(
            t.test_pages.0[self.index] & page::OUTPUT_ADDRESS_MASK,
            self.frame.address as u64
        );
        t.test_pages.0[self.index] = 0;
        cpu::invalidate();
    }
}
static HEAP_BASE: AtomicUsize = AtomicUsize::new(0);
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
