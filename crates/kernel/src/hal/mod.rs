use core::ptr::{read_volatile, write_volatile};
pub struct Registers {
    base: usize,
    size: usize,
}
impl Registers {
    /// # Safety
    /// INV-MMIO: region is validated device memory, mapped for this kernel's lifetime.
    pub unsafe fn new(base: usize, size: usize) -> Self {
        Self { base, size }
    }
    fn address(&self, offset: usize) -> *mut u32 {
        let width = core::mem::size_of::<u32>();
        assert!(
            offset.is_multiple_of(width)
                && offset.checked_add(width).is_some_and(|v| v <= self.size)
        );
        self.base
            .checked_add(offset)
            .expect("MMIO address overflow") as *mut u32
    }
    pub fn read(&self, offset: usize) -> u32 {
        // SAFETY: INV-MMIO: construction guarantees validity; address checks width/alignment.
        unsafe { read_volatile(self.address(offset)) }
    }
    pub fn write(&self, offset: usize, value: u32) {
        // SAFETY: INV-MMIO: bounded register access, no ordinary reference to device memory.
        unsafe {
            write_volatile(self.address(offset), value);
        }
    }
}
