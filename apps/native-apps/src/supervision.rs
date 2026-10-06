//! Application policy for the existing native lifecycle response.
//! Only an actual terminal response permits replacement; client claims do not.
pub fn needs_replacement(reply: [u64; 5]) -> Result<bool, u64> {
    if reply[0] != 0 {
        return Err(reply[0]);
    }
    match reply[1] {
        0 => Ok(false),
        1..=4 => Ok(true),
        _ => Err(1),
    }
}
pub fn fresh_binding(old_instance: u64, reply: [u64; 4]) -> Result<(), u64> {
    if reply[3] != 0 {
        return Err(reply[3]);
    }
    if reply[0] == 0 || reply[1] == 0 || reply[1] == old_instance || reply[2] != 0 {
        return Err(1);
    }
    Ok(())
}

#[cfg(target_arch = "aarch64")]
pub fn recover_service(old: [u64; 5]) -> Result<[u64; 5], u64> {
    use native_userspace::{CounterRequest, counter_value, native};
    let observed = native::lifecycle(2, 0, old[1], 0);
    if !needs_replacement(observed)? {
        return Err(13);
    }
    let (now, hz) = native::clock();
    let until = now.checked_add(hz / 128).ok_or(1u64)?;
    while native::clock().0 < until {
        core::hint::spin_loop();
    }
    let fresh = native::lifecycle(3, 0, old[1], 0);
    if fresh[0] != 0 {
        return Err(fresh[0]);
    }
    if fresh[1] == old[1] || fresh[4] == old[4] {
        return Err(2);
    }
    let (bytes, n) = native::rpc(fresh[2], 1, &CounterRequest::Get.encode()).map_err(|_| 1u64)?;
    if counter_value(&bytes[..n]) != Ok(0) {
        return Err(1);
    }
    Ok(fresh)
}
