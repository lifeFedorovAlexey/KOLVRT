#![no_std]
#![no_main]
mod abi_steps;
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, feedback: u64, _argument: u64) -> ! {
    abi_steps::exercise(receiver, feedback);
    // SAFETY: external isolated EL0 actor deliberately executes an illegal instruction.
    unsafe {
        core::arch::asm!(".inst 0", options(noreturn));
    }
}
