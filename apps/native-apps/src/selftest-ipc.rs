#![no_std]
#![no_main]
mod client;
native_userspace::entry!();
#[unsafe(no_mangle)]
pub extern "C" fn native_main(receiver: u64, feedback: u64, argument: u64) -> ! {
    client::run(receiver, feedback, argument, 3)
}
