#![no_std]
#![no_main]
mod abi_steps;
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, feedback: u64, _argument: u64) -> ! {
    abi_steps::exercise(receiver, feedback);
    native_userspace::native::exit(0)
}
